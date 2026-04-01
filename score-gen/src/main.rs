use anyhow::{Context, Result};
use frontend::archetypes::RunnableArchetypeList;
use frontend::instances::InstanceList;
use frontend::interfaces::InterfaceList;
use frontend::internal_state::InternalStateDefinition;
use frontend::parameters::ParameterDefinition;
use frontend::system_manifest::SystemManifest;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

struct DiscoveredFiles {
    archetypes: Vec<PathBuf>,
    interfaces: Vec<PathBuf>,
    instances: Vec<PathBuf>,
    system_name: String,
}

/// Discover YAML files from a system manifest.
fn discover_files(manifest_path: &PathBuf) -> Result<DiscoveredFiles> {
    let manifest = SystemManifest::from_file(manifest_path)?;
    let base_dir = manifest_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine parent directory of manifest"))?;
    Ok(DiscoveredFiles {
        archetypes: manifest.archetype_paths(base_dir),
        interfaces: manifest.interface_paths(base_dir),
        instances: manifest.instance_paths(base_dir),
        system_name: manifest.system_name,
    })
}

struct CliArgs {
    manifest_path: PathBuf,
    graphviz_path: Option<PathBuf>,
}

fn parse_args() -> Result<CliArgs> {
    let args: Vec<String> = std::env::args().collect();
    let bwd = std::env::var("BUILD_WORKING_DIRECTORY").ok();

    let resolve = |p: &str| -> PathBuf {
        match &bwd {
            Some(dir) => PathBuf::from(dir).join(p),
            None => PathBuf::from(p),
        }
    };

    let mut graphviz_path: Option<PathBuf> = None;
    let mut positional: Vec<String> = Vec::new();

    for arg in args.iter().skip(1) {
        if arg == "-h" || arg == "--help" {
            print_help();
            std::process::exit(0);
        } else if let Some(path) = arg.strip_prefix("--graphviz=") {
            graphviz_path = Some(resolve(path));
        } else if arg.starts_with('-') {
            anyhow::bail!("Unknown option: {}", arg);
        } else {
            positional.push(arg.clone());
        }
    }

    if positional.is_empty() {
        print_help();
        std::process::exit(1);
    }

    let manifest_path = resolve(&positional[0]);

    Ok(CliArgs {
        manifest_path,
        graphviz_path,
    })
}

fn print_help() {
    println!("scoregen - Score code generation");
    println!();
    println!("USAGE:");
    println!("    scoregen [OPTIONS] <manifest.yaml>");
    println!();
    println!("ARGS:");
    println!("    <manifest.yaml>           Path to the system manifest YAML file.");
    println!("                              All files referenced in the manifest are");
    println!("                              resolved relative to the manifest's directory.");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help                Print this help message");
    println!("    --graphviz=<path>         Generate a Graphviz DOT file of the dataflow graph");
    println!();
    println!("EXAMPLES:");
    println!("    bazel run //:scoregen -- $PWD/graph_yaml/my_manifest.yaml");
    println!("    bazel run //:scoregen -- --graphviz=$PWD/graph.gv $PWD/graph_yaml/system_manifest.yaml");
    println!("    bazel run @score-gen//:scoregen -- $PWD/graph_yaml/my_manifest.yaml");
}

fn run() -> Result<()> {
    let cli = parse_args()?;

    if !cli.manifest_path.is_file() {
        anyhow::bail!("{} is not a file", cli.manifest_path.display());
    }

    let discovered = discover_files(&cli.manifest_path)?;

    println!("System: {}", discovered.system_name);

    // Parse interfaces
    let mut all_interfaces: Vec<InterfaceList> = Vec::new();
    for path in &discovered.interfaces {
        let list = InterfaceList::from_file(path)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        println!(
            "Parsed {} ({} structs, {} enums, {} constexprs in namespace '{}')",
            path.display(),
            list.structs.len(),
            list.enums.len(),
            list.constexprs.len(),
            list.namespace
        );
        all_interfaces.push(list);
    }

    // Parse archetypes
    let mut all_archetypes: Vec<RunnableArchetypeList> = Vec::new();
    for path in &discovered.archetypes {
        let list = RunnableArchetypeList::from_file(path)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        println!(
            "Parsed {} ({} runnables in namespace '{}')",
            path.display(),
            list.runnables.len(),
            list.namespace
        );
        all_archetypes.push(list);
    }

    // Parse parameters referenced by archetypes
    let manifest_dir = cli
        .manifest_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine manifest directory"))?;
    let mut all_parameters: HashMap<String, ParameterDefinition> = HashMap::new();
    let mut all_internal_states: HashMap<String, InternalStateDefinition> = HashMap::new();
    for list in &all_archetypes {
        for r in &list.runnables {
            if let Some(ref param_path) = r.parameter_header {
                let full_path = manifest_dir.join(param_path);
                let def = ParameterDefinition::from_file(&full_path)
                    .with_context(|| format!("Failed to parse parameters for {}", r.unique_name))?;
                println!(
                    "Parsed {} ({} parameters for {}::{})",
                    full_path.display(),
                    def.members.len(),
                    list.namespace,
                    r.unique_name
                );
                all_parameters.insert(r.unique_name.clone(), def);
            }
            if let Some(ref state_path) = r.internal_state_header {
                let full_path = manifest_dir.join(state_path);
                let def = InternalStateDefinition::from_file(&full_path)
                    .with_context(|| format!("Failed to parse internal state for {}", r.unique_name))?;
                println!(
                    "Parsed {} ({} state members for {}::{})",
                    full_path.display(),
                    def.members.len(),
                    list.namespace,
                    r.unique_name
                );
                all_internal_states.insert(r.unique_name.clone(), def);
            }
        }
    }

    // Parse instance files
    let mut all_instances: Vec<InstanceList> = Vec::new();
    for path in &discovered.instances {
        let list = InstanceList::from_file(path)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        println!(
            "  {} instance(s) in namespace {}",
            list.instances.len(),
            list.namespace
        );
        all_instances.push(list);
    }

    println!(
        "\nTotal: {} interface file(s) ({} structs, {} enums), {} archetype file(s) ({} runnables), {} parameter file(s), {} internal state file(s), {} instance file(s) ({} instances)",
        all_interfaces.len(),
        all_interfaces.iter().map(|i| i.structs.len()).sum::<usize>(),
        all_interfaces.iter().map(|i| i.enums.len()).sum::<usize>(),
        all_archetypes.len(),
        all_archetypes.iter().map(|a| a.runnables.len()).sum::<usize>(),
        all_parameters.len(),
        all_internal_states.len(),
        all_instances.len(),
        all_instances.iter().map(|i| i.instances.len()).sum::<usize>()
    );

    for list in &all_archetypes {
        for r in &list.runnables {
            println!(
                "  {}::{} ({})",
                list.namespace, r.unique_name, r.runnabletype
            );
        }
    }

    // Generate graphviz output
    if let Some(ref gv_path) = cli.graphviz_path {
        let flat: Vec<(String, &frontend::archetypes::RunnableArchetypeDescription)> =
            all_archetypes
                .iter()
                .flat_map(|list| {
                    list.runnables
                        .iter()
                        .map(move |r| (list.namespace.0.clone(), r))
                })
                .collect();

        let dot = util::graphviz::generate_dot(&discovered.system_name, &flat);
        let mut file = std::fs::File::create(gv_path)
            .with_context(|| format!("Failed to create {}", gv_path.display()))?;
        file.write_all(dot.as_bytes())?;
        println!("\nGraphviz written to {}", gv_path.display());
        println!("Render with: dot -Tpng {} -o graph.png", gv_path.display());
    }

    // Generate C++ skeleton code
    let src_gen_dir = manifest_dir
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine parent of manifest directory"))?
        .join("src-gen");
    println!("\nGenerating C++ skeletons in {}", src_gen_dir.display());
    let generated = backend::codegen::generate(&src_gen_dir, &all_archetypes, &all_parameters, &all_internal_states, &all_interfaces, None)?;
    for name in &generated {
        println!("  Generated {}", name);
    }
    println!("Generated {} skeleton(s)", generated.len());

    // Generate C++ interface headers
    let interfaces_dir = src_gen_dir.join("interfaces");
    println!("\nGenerating C++ interfaces in {}", interfaces_dir.display());
    let generated_ifaces = backend::codegen::generate_interfaces(&interfaces_dir, &all_interfaces, None)?;
    for name in &generated_ifaces {
        println!("  Generated {}", name);
    }
    println!("Generated {} interface(s)", generated_ifaces.len());

    // Generate task binaries for instances
    if !all_instances.is_empty() {
        println!("\nGenerating task binaries in {}", src_gen_dir.display());
        let generated_tasks = backend::codegen::generate_tasks(
            &src_gen_dir,
            &all_instances,
            &all_archetypes,
            &all_parameters,
            &all_internal_states,
            None,
        )?;
        for name in &generated_tasks {
            println!("  Generated {}", name);
        }
        println!("Generated {} task file(s)", generated_tasks.len());
    }

    // Generate MODULE.bazel at the project root
    let project_root = manifest_dir
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine project root"))?;

    // Discover score-runtime: try CARGO_MANIFEST_DIR first (cargo run),
    // then search upward from the manifest for a sibling score-runtime directory.
    let score_runtime_dir = {
        let from_cargo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(|p| p.join("score-runtime"))
            .filter(|p| p.is_dir());
        if let Some(dir) = from_cargo {
            dir
        } else {
            // Walk up from manifest_dir looking for a directory containing score-runtime
            let abs_manifest_dir = std::fs::canonicalize(manifest_dir)
                .with_context(|| format!("Failed to canonicalize {}", manifest_dir.display()))?;
            let mut search = abs_manifest_dir.as_path();
            loop {
                let candidate = search.join("score-runtime");
                if candidate.is_dir() {
                    break candidate;
                }
                search = search.parent().ok_or_else(|| {
                    anyhow::anyhow!(
                        "score-runtime not found. Searched upward from {}",
                        abs_manifest_dir.display()
                    )
                })?;
            }
        }
    };

    // Compute relative path from project_root to score-runtime
    let abs_root = std::fs::canonicalize(project_root)
        .with_context(|| format!("Failed to canonicalize {}", project_root.display()))?;
    let abs_runtime = std::fs::canonicalize(&score_runtime_dir)
        .with_context(|| format!("Failed to canonicalize {}", score_runtime_dir.display()))?;
    let rel_runtime = relative_path(&abs_root, &abs_runtime);

    let module_name = discovered.system_name.replace(' ', "-").to_lowercase();
    backend::codegen::generate_module_bazel(project_root, &module_name, &rel_runtime, None)?;
    println!("\nGenerated MODULE.bazel at {}", project_root.join("MODULE.bazel").display());

    Ok(())
}

/// Compute a relative path from `base` to `target`.
fn relative_path(base: &std::path::Path, target: &std::path::Path) -> String {
    let base_parts: Vec<_> = base.components().collect();
    let target_parts: Vec<_> = target.components().collect();
    let common = base_parts
        .iter()
        .zip(target_parts.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let ups = base_parts.len() - common;
    let mut result = PathBuf::new();
    for _ in 0..ups {
        result.push("..");
    }
    for part in &target_parts[common..] {
        result.push(part);
    }
    result.to_string_lossy().to_string()
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

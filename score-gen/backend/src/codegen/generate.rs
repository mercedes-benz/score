use anyhow::{Context, Result};
use frontend::archetypes::RunnableArchetypeList;
use frontend::interfaces::InterfaceList;
use frontend::internal_state::InternalStateDefinition;
use frontend::parameters::ParameterDefinition;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use super::render;
use super::runnable::RunnableContext;
use super::to_snake_case;

const DEFAULT_HEADER_TEMPLATE: &str = include_str!("../../templates/runnable.hpp.j2");
const DEFAULT_SOURCE_TEMPLATE: &str = include_str!("../../templates/runnable.cpp.j2");
const DEFAULT_BUILD_TEMPLATE: &str = include_str!("../../templates/BUILD.bazel.j2");
const DEFAULT_TASK_TEST_TEMPLATE: &str = include_str!("../../templates/task_test.cpp.j2");

/// Template context for generating a BUILD.bazel file per runnable.
#[derive(Serialize)]
pub(crate) struct BuildContext {
    pub lib_name: String,
    pub has_parameters: bool,
    pub has_internal_state: bool,
    pub has_inputs: bool,
    pub has_outputs: bool,
}

/// Loaded template sources for code generation.
pub(crate) struct Templates {
    pub header: String,
    pub source: String,
    pub build: String,
    pub unit_test: String,
}

/// Load templates from a directory if provided, or use built-in defaults.
pub(crate) fn load_templates(template_dir: Option<&Path>) -> Result<Templates> {
    if let Some(dir) = template_dir {
        let hpp_path = dir.join("runnable.hpp.j2");
        let cpp_path = dir.join("runnable.cpp.j2");
        let build_path = dir.join("BUILD.bazel.j2");
        let test_path = dir.join("task_test.cpp.j2");

        let header = fs::read_to_string(&hpp_path)
            .with_context(|| format!("Failed to read template {}", hpp_path.display()))?;
        let source = fs::read_to_string(&cpp_path)
            .with_context(|| format!("Failed to read template {}", cpp_path.display()))?;
        let build = fs::read_to_string(&build_path)
            .with_context(|| format!("Failed to read template {}", build_path.display()))?;
        let unit_test = fs::read_to_string(&test_path)
            .with_context(|| format!("Failed to read template {}", test_path.display()))?;

        Ok(Templates { header, source, build, unit_test })
    } else {
        Ok(Templates {
            header: DEFAULT_HEADER_TEMPLATE.to_string(),
            source: DEFAULT_SOURCE_TEMPLATE.to_string(),
            build: DEFAULT_BUILD_TEMPLATE.to_string(),
            unit_test: DEFAULT_TASK_TEST_TEMPLATE.to_string(),
        })
    }
}

/// Generate C++ skeleton files for all runnables into the output directory.
///
/// If `template_dir` is `Some`, templates are loaded from that directory.
/// Otherwise, the built-in default templates are used.
///
/// `parameters` maps runnable unique_name to its parsed ParameterDefinition.
/// `internal_states` maps runnable unique_name to its parsed InternalStateDefinition.
/// `interface_lists` are used to validate that all input type_names are declared.
pub fn generate(
    output_dir: &Path,
    archetype_lists: &[RunnableArchetypeList],
    parameters: &HashMap<String, ParameterDefinition>,
    internal_states: &HashMap<String, InternalStateDefinition>,
    interface_lists: &[InterfaceList],
    template_dir: Option<&Path>,
) -> Result<Vec<String>> {
    // Build set of known interface types (fully-qualified: "namespace::StructName")
    let known_types: HashSet<String> = interface_lists
        .iter()
        .flat_map(|list| {
            list.structs
                .iter()
                .map(move |s| format!("{}::{}", list.namespace, s.name))
        })
        .collect();

    // Validate all input and output type_names against known interface types
    for list in archetype_lists {
        for runnable in &list.runnables {
            for input in &runnable.inputs {
                if !known_types.contains(&input.type_name) {
                    anyhow::bail!(
                        "Dangling input type '{}' on input '{}' of runnable '{}': \
                         type not declared in any interface file",
                        input.type_name,
                        input.unique_name,
                        runnable.unique_name,
                    );
                }
            }
            for output in &runnable.outputs {
                if !known_types.contains(&output.type_name) {
                    anyhow::bail!(
                        "Dangling output type '{}' on output '{}' of runnable '{}': \
                         type not declared in any interface file",
                        output.type_name,
                        output.unique_name,
                        runnable.unique_name,
                    );
                }
            }
        }
    }

    let templates = load_templates(template_dir)?;
    let consumer_map = super::runnable::build_consumer_map(archetype_lists);
    let mut generated = Vec::new();

    for list in archetype_lists {
        let ns = &list.namespace.0;
        let snake_ns = to_snake_case(ns);
        let ns_dir = output_dir.join(&snake_ns);
        fs::create_dir_all(&ns_dir)
            .with_context(|| format!("Failed to create directory {}", ns_dir.display()))?;

        for runnable in &list.runnables {
            let ctx = RunnableContext::from(ns, runnable, Some(&consumer_map));

            let runnable_dir = ns_dir.join(&ctx.snake_name);
            let include_dir = runnable_dir.join("include");
            let srcgen_dir = runnable_dir.join("src");
            fs::create_dir_all(&include_dir)
                .with_context(|| format!("Failed to create directory {}", include_dir.display()))?;
            fs::create_dir_all(&srcgen_dir)
                .with_context(|| format!("Failed to create directory {}", srcgen_dir.display()))?;

            let hpp_path = include_dir.join(format!("{}.hpp.skeleton", ctx.snake_name));
            let cpp_path = srcgen_dir.join(format!("{}.cpp.skeleton", ctx.snake_name));

            let header = render(&templates.header, &ctx)
                .with_context(|| format!("Failed to render header for {}", runnable.unique_name))?;
            let source = render(&templates.source, &ctx)
                .with_context(|| format!("Failed to render source for {}", runnable.unique_name))?;

            fs::write(&hpp_path, &header)
                .with_context(|| format!("Failed to write {}", hpp_path.display()))?;
            fs::write(&cpp_path, &source)
                .with_context(|| format!("Failed to write {}", cpp_path.display()))?;

            // Generate BUILD.bazel for this runnable
            let build_ctx = BuildContext {
                lib_name: ctx.snake_name.clone(),
                has_parameters: parameters.contains_key(&runnable.unique_name),
                has_internal_state: internal_states.contains_key(&runnable.unique_name),
                has_inputs: !runnable.inputs.is_empty(),
                has_outputs: !runnable.outputs.is_empty(),
            };
            let build_content = render(&templates.build, &build_ctx)
                .with_context(|| format!("Failed to render BUILD for {}", runnable.unique_name))?;
            let build_path = runnable_dir.join("BUILD.bazel.skeleton");
            fs::write(&build_path, &build_content)
                .with_context(|| format!("Failed to write {}", build_path.display()))?;

            // Generate unit test
            let test_dir = runnable_dir.join("test");
            fs::create_dir_all(&test_dir)
                .with_context(|| format!("Failed to create directory {}", test_dir.display()))?;
            let test_content = render(&templates.unit_test, &ctx)
                .with_context(|| format!("Failed to render unit test for {}", runnable.unique_name))?;
            let test_path = test_dir.join("unit_test.cpp.skeleton");
            fs::write(&test_path, &test_content)
                .with_context(|| format!("Failed to write {}", test_path.display()))?;

            generated.push(format!("{}::{}", ns, runnable.unique_name));
        }
    }

    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use frontend::archetypes::RunnableArchetypeList;

    #[test]
    fn test_generate_writes_files() {
        let dir = std::env::temp_dir().join("scoregen_test_generate_j2");
        let _ = fs::remove_dir_all(&dir);

        let list = RunnableArchetypeList::from_str(
            r#"
namespace: TestNS
runnables:
  - unique_name: MyRunnable
    runnabletype: CPU
    build_information:
      bazel_target: "//test:test"
"#,
        )
        .unwrap();

        let generated = generate(&dir, &[list], &HashMap::new(), &HashMap::new(), &[], None).unwrap();
        assert_eq!(generated.len(), 1);
        assert_eq!(generated[0], "TestNS::MyRunnable");

        let hpp = fs::read_to_string(dir.join("test_ns/my_runnable/include/my_runnable.hpp.skeleton")).unwrap();
        assert!(hpp.contains("namespace my_runnable"));

        let cpp = fs::read_to_string(dir.join("test_ns/my_runnable/src/my_runnable.cpp.skeleton")).unwrap();
        assert!(cpp.contains("namespace my_runnable"));

        let build = fs::read_to_string(dir.join("test_ns/my_runnable/BUILD.bazel.skeleton")).unwrap();
        assert!(build.contains("name = \"my_runnable\""));
        assert!(build.contains("@score-runtime//base"));
        assert!(build.contains("cc_test"));
        assert!(build.contains("name = \"unit_test\""));
        assert!(build.contains("@googletest//:gtest_main"));

        // Unit test generated in archetype test/ directory
        let test_cpp = fs::read_to_string(dir.join("test_ns/my_runnable/test/unit_test.cpp.skeleton")).unwrap();
        assert!(test_cpp.contains("#include <gtest/gtest.h>"));
        assert!(test_cpp.contains("class MyRunnableTest"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generate_multiple_runnables() {
        let dir = std::env::temp_dir().join("scoregen_test_multi_j2");
        let _ = fs::remove_dir_all(&dir);

        let list = RunnableArchetypeList::from_str(
            r#"
namespace: Multi
runnables:
  - unique_name: Alpha
    runnabletype: CPU
    build_information:
      bazel_target: "//a:a"
  - unique_name: BetaGamma
    runnabletype: GPU
    build_information:
      bazel_target: "//b:b"
"#,
        )
        .unwrap();

        let generated = generate(&dir, &[list], &HashMap::new(), &HashMap::new(), &[], None).unwrap();
        assert_eq!(generated.len(), 2);

        assert!(dir.join("multi/alpha/include/alpha.hpp.skeleton").exists());
        assert!(dir.join("multi/alpha/src/alpha.cpp.skeleton").exists());
        assert!(dir.join("multi/beta_gamma/include/beta_gamma.hpp.skeleton").exists());
        assert!(dir.join("multi/beta_gamma/src/beta_gamma.cpp.skeleton").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generate_with_custom_templates() {
        let tmpl_dir = std::env::temp_dir().join("scoregen_test_custom_tmpl");
        let out_dir = std::env::temp_dir().join("scoregen_test_custom_out");
        let _ = fs::remove_dir_all(&tmpl_dir);
        let _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(&tmpl_dir).unwrap();

        // Custom templates with different content
        fs::write(
            tmpl_dir.join("runnable.hpp.j2"),
            "// Custom header for {{ class_name }} in {{ namespace }}\n",
        )
        .unwrap();
        fs::write(
            tmpl_dir.join("runnable.cpp.j2"),
            "// Custom source for {{ class_name }}\n",
        )
        .unwrap();
        fs::write(
            tmpl_dir.join("BUILD.bazel.j2"),
            "# Custom BUILD for {{ lib_name }}\n",
        )
        .unwrap();
        fs::write(
            tmpl_dir.join("task_test.cpp.j2"),
            "// Custom unit test for {{ class_name }}\n",
        )
        .unwrap();

        let list = RunnableArchetypeList::from_str(
            r#"
namespace: NS
runnables:
  - unique_name: Foo
    runnabletype: CPU
    build_information:
      bazel_target: "//f:f"
"#,
        )
        .unwrap();

        let generated = generate(&out_dir, &[list], &HashMap::new(), &HashMap::new(), &[], Some(&tmpl_dir)).unwrap();
        assert_eq!(generated.len(), 1);

        let hpp = fs::read_to_string(out_dir.join("ns/foo/include/foo.hpp.skeleton")).unwrap();
        assert!(hpp.contains("Custom header for Foo in ns"));

        let cpp = fs::read_to_string(out_dir.join("ns/foo/src/foo.cpp.skeleton")).unwrap();
        assert!(cpp.contains("Custom source for Foo"));

        let _ = fs::remove_dir_all(&tmpl_dir);
        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn test_generate_with_parameters() {
        let dir = std::env::temp_dir().join("scoregen_test_params");
        let _ = fs::remove_dir_all(&dir);

        let list = RunnableArchetypeList::from_str(
            r#"
namespace: TestNS
runnables:
  - unique_name: MyRunnable
    runnabletype: CPU
    parameter_header: params/my.yaml
    build_information:
      bazel_target: "//test:test"
"#,
        )
        .unwrap();

        let param_def = ParameterDefinition::from_str(
            r#"
name: MyRunnableParameters
top_level: true
members:
  - name: speed
    member_type: float
    default: 1.0
"#,
        )
        .unwrap();

        let mut params = HashMap::new();
        params.insert("MyRunnable".to_string(), param_def);

        let generated = generate(&dir, &[list], &params, &HashMap::new(), &[], None).unwrap();
        assert_eq!(generated.len(), 1);

        // Parameters are no longer generated in archetype directory
        assert!(!dir.join("test_ns/my_runnable/include/parameters.hpp").exists());

        // Runnable header forward-declares the parameter struct
        let hpp = fs::read_to_string(
            dir.join("test_ns/my_runnable/include/my_runnable.hpp.skeleton"),
        )
        .unwrap();
        assert!(hpp.contains("struct MyRunnableParameters;"));
        assert!(!hpp.contains("#include \"parameters.hpp\""));
        assert!(hpp.contains("on_init(const MyRunnableParameters& parameters)"));
        assert!(hpp.contains("on_update(const MyRunnableParameters& parameters)"));

        // Check runnable source uses parameters
        let cpp = fs::read_to_string(
            dir.join("test_ns/my_runnable/src/my_runnable.cpp.skeleton"),
        )
        .unwrap();
        assert!(cpp.contains("on_init(const MyRunnableParameters& parameters)"));
        assert!(cpp.contains("on_update(const MyRunnableParameters& parameters)"));

        // Check BUILD has parameter dependency
        let build = fs::read_to_string(
            dir.join("test_ns/my_runnable/BUILD.bazel.skeleton"),
        )
        .unwrap();
        assert!(build.contains("//src-gen/parameter:parameter"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generate_with_internal_state() {
        let dir = std::env::temp_dir().join("scoregen_test_state");
        let _ = fs::remove_dir_all(&dir);

        let list = RunnableArchetypeList::from_str(
            r#"
namespace: TestNS
runnables:
  - unique_name: MyRunnable
    runnabletype: CPU
    internal_state_header: states/my.yaml
    build_information:
      bazel_target: "//test:test"
"#,
        )
        .unwrap();

        let state_def = InternalStateDefinition::from_str(
            r#"
name: MyState
top_level: true
members:
  - name: counter
    member_type: uint64_t
    default: 0
"#,
        )
        .unwrap();

        let mut states = HashMap::new();
        states.insert("MyRunnable".to_string(), state_def);

        let generated = generate(&dir, &[list], &HashMap::new(), &states, &[], None).unwrap();
        assert_eq!(generated.len(), 1);

        // Internal state is no longer generated in archetype directory
        assert!(!dir.join("test_ns/my_runnable/include/internal_state.hpp").exists());

        // Runnable header forward-declares the internal state struct
        let hpp = fs::read_to_string(
            dir.join("test_ns/my_runnable/include/my_runnable.hpp.skeleton"),
        )
        .unwrap();
        assert!(hpp.contains("struct MyRunnableInternalState;"));
        assert!(!hpp.contains("#include \"internal_state.hpp\""));
        assert!(hpp.contains("on_init(MyRunnableInternalState& state)"));
        assert!(hpp.contains("on_update(MyRunnableInternalState& state)"));

        // Check runnable source uses internal state
        let cpp = fs::read_to_string(
            dir.join("test_ns/my_runnable/src/my_runnable.cpp.skeleton"),
        )
        .unwrap();
        assert!(cpp.contains("on_init(MyRunnableInternalState& state)"));
        assert!(cpp.contains("on_update(MyRunnableInternalState& state)"));

        // Check BUILD has internal_state dependency
        let build = fs::read_to_string(
            dir.join("test_ns/my_runnable/BUILD.bazel.skeleton"),
        )
        .unwrap();
        assert!(build.contains("//src-gen/internal_state:internal_state"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generate_with_inputs() {
        let dir = std::env::temp_dir().join("scoregen_test_inputs");
        let _ = fs::remove_dir_all(&dir);

        let list = RunnableArchetypeList::from_str(
            r#"
namespace: TestNS
runnables:
  - unique_name: Consumer
    runnabletype: CPU
    inputs:
      - unique_name: sensor_data
        type_name: "test::ns::SensorData"
        n_samples_max: 3
    build_information:
      bazel_target: "//test:test"
"#,
        )
        .unwrap();

        let iface = InterfaceList::from_str(
            r#"
namespace: "test::ns"
structs:
  - name: SensorData
    top_level: true
    members:
      - name: value
        type: float
        default: 0.0
"#,
        )
        .unwrap();

        let generated = generate(&dir, &[list], &HashMap::new(), &HashMap::new(), &[iface], None).unwrap();
        assert_eq!(generated.len(), 1);

        let hpp = fs::read_to_string(dir.join("test_ns/consumer/include/consumer.hpp.skeleton")).unwrap();
        assert!(hpp.contains("#include <deque>"));
        assert!(hpp.contains("struct Inputs"));
        assert!(hpp.contains("std::deque<const test::ns::SensorData*> sensor_data{sensor_data_capacity, nullptr};"));
        assert!(hpp.contains("static constexpr std::size_t sensor_data_capacity = 3;"));
        assert!(hpp.contains("const Inputs& inputs"));

        let build = fs::read_to_string(dir.join("test_ns/consumer/BUILD.bazel.skeleton")).unwrap();
        assert!(build.contains("//src-gen/interfaces:interfaces"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generate_rejects_dangling_input_type() {
        let dir = std::env::temp_dir().join("scoregen_test_dangling");
        let _ = fs::remove_dir_all(&dir);

        let list = RunnableArchetypeList::from_str(
            r#"
namespace: TestNS
runnables:
  - unique_name: Bad
    runnabletype: CPU
    inputs:
      - unique_name: missing_data
        type_name: "unknown::ns::MissingType"
        n_samples_max: 1
    build_information:
      bazel_target: "//test:test"
"#,
        )
        .unwrap();

        let result = generate(&dir, &[list], &HashMap::new(), &HashMap::new(), &[], None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Dangling input type"));
        assert!(err.contains("unknown::ns::MissingType"));
        assert!(err.contains("Bad"));

        let _ = fs::remove_dir_all(&dir);
    }
}

use anyhow::{Context, Result};
use frontend::archetypes::RunnableArchetypeList;
use frontend::instances::InstanceList;
use frontend::internal_state::InternalStateDefinition;
use frontend::parameters::ParameterDefinition;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::internal_state::InternalStateContext;
use super::render;
use super::to_snake_case;
use super::DEFAULT_PARAMS_TEMPLATE;
use super::DEFAULT_STATE_TEMPLATE;
use task::{TaskBuildContext, TaskBuildEntry, TaskContext, TaskParameterContext};

use super::task;

const DEFAULT_TASK_TEMPLATE: &str = include_str!("../../templates/task.cpp.j2");
const DEFAULT_TASK_BUILD_TEMPLATE: &str = include_str!("../../templates/task_BUILD.bazel.j2");

/// Generate task.cpp and BUILD.bazel for each runnable instance.
///
/// `output_dir` is the src-gen root. Tasks are placed under
/// `<instance_namespace>/<archetype_snake_name>/`.
///
/// `archetype_lists` are needed to resolve archetype references.
/// `parameters` maps archetype unique_name to its ParameterDefinition.
/// `internal_states` maps archetype unique_name to its InternalStateDefinition.
pub fn generate_tasks(
    output_dir: &Path,
    instance_lists: &[InstanceList],
    archetype_lists: &[RunnableArchetypeList],
    parameters: &HashMap<String, ParameterDefinition>,
    internal_states: &HashMap<String, InternalStateDefinition>,
    template_dir: Option<&Path>,
) -> Result<Vec<String>> {
    let task_tmpl = if let Some(dir) = template_dir {
        let p = dir.join("task.cpp.j2");
        fs::read_to_string(&p)
            .with_context(|| format!("Failed to read template {}", p.display()))?
    } else {
        DEFAULT_TASK_TEMPLATE.to_string()
    };
    let task_build_tmpl = if let Some(dir) = template_dir {
        let p = dir.join("task_BUILD.bazel.j2");
        fs::read_to_string(&p)
            .with_context(|| format!("Failed to read template {}", p.display()))?
    } else {
        DEFAULT_TASK_BUILD_TEMPLATE.to_string()
    };
    let params_tmpl = if let Some(dir) = template_dir {
        let p = dir.join("parameters.hpp.j2");
        fs::read_to_string(&p)
            .with_context(|| format!("Failed to read template {}", p.display()))?
    } else {
        DEFAULT_PARAMS_TEMPLATE.to_string()
    };
    let state_tmpl = if let Some(dir) = template_dir {
        let p = dir.join("internal_state.hpp.j2");
        fs::read_to_string(&p)
            .with_context(|| format!("Failed to read template {}", p.display()))?
    } else {
        DEFAULT_STATE_TEMPLATE.to_string()
    };

    let mut generated = Vec::new();

    // Group instances by namespace for a single BUILD.bazel per namespace
    let mut ns_tasks: HashMap<String, Vec<(String, String, String, bool)>> = HashMap::new();

    // Create the parameter directory for all parameter headers
    let param_dir = output_dir.join("parameter");
    fs::create_dir_all(&param_dir)
        .with_context(|| format!("Failed to create directory {}", param_dir.display()))?;

    // Create the internal_state directory for all internal state headers
    let state_dir = output_dir.join("internal_state");
    fs::create_dir_all(&state_dir)
        .with_context(|| format!("Failed to create directory {}", state_dir.display()))?;

    for inst_list in instance_lists {
        let inst_ns_snake = to_snake_case(&inst_list.namespace);

        for instance in &inst_list.instances {
            // Find the archetype this instance references
            let archetype = archetype_lists
                .iter()
                .find_map(|list| list.find_archetype(&instance.archetype_reference))
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Archetype '{}' not found for instance '{}'",
                        instance.archetype_reference,
                        instance.unique_name
                    )
                })?;

            // Find archetype namespace
            let arch_ns = archetype_lists
                .iter()
                .find(|list| list.find_archetype(&instance.archetype_reference).is_some())
                .map(|list| list.namespace.0.clone())
                .unwrap();

            let has_params = archetype.parameter_header.is_some();
            let has_state = archetype.internal_state_header.is_some();
            let arch_snake = to_snake_case(&archetype.unique_name);

            let task_ctx = TaskContext::new(
                &arch_ns,
                &inst_list.namespace,
                &archetype.unique_name,
                has_params,
                has_state,
                !archetype.inputs.is_empty(),
                !archetype.outputs.is_empty(),
            );

            // Output: src-gen/<inst_ns>/src/<arch_snake>.cpp
            let ns_dir = output_dir.join(&inst_ns_snake);
            let src_dir = ns_dir.join("src");
            fs::create_dir_all(&src_dir)
                .with_context(|| format!("Failed to create directory {}", src_dir.display()))?;

            // Render and write <arch_snake>.cpp
            let task_cpp = render(&task_tmpl, &task_ctx)
                .with_context(|| format!("Failed to render task for {}", instance.unique_name))?;
            let task_path = src_dir.join(format!("{}.cpp", &arch_snake));
            fs::write(&task_path, &task_cpp)
                .with_context(|| format!("Failed to write {}", task_path.display()))?;

            // Generate parameter header in src-gen/parameter/
            if has_params {
                if let Some(param_def) = parameters.get(&archetype.unique_name) {
                    let task_param_ctx = TaskParameterContext::new(
                        &archetype.unique_name,
                        param_def,
                        &instance.parameter_overrides,
                    );
                    let param_hpp = render(&params_tmpl, &task_param_ctx)
                        .with_context(|| {
                            format!(
                                "Failed to render parameter header for {}",
                                instance.unique_name
                            )
                        })?;
                    let param_path =
                        param_dir.join(format!("{}_parameter.hpp", &arch_snake));
                    fs::write(&param_path, &param_hpp).with_context(|| {
                        format!("Failed to write {}", param_path.display())
                    })?;
                }
            }

            // Generate internal state header in src-gen/internal_state/
            if has_state {
                if let Some(state_def) = internal_states.get(&archetype.unique_name) {
                    let state_ctx = InternalStateContext::from(
                        &arch_ns,
                        archetype,
                        state_def,
                    );
                    let state_hpp = render(&state_tmpl, &state_ctx)
                        .with_context(|| {
                            format!(
                                "Failed to render internal state header for {}",
                                instance.unique_name
                            )
                        })?;
                    let state_path =
                        state_dir.join(format!("{}_internal_state.hpp", &arch_snake));
                    fs::write(&state_path, &state_hpp).with_context(|| {
                        format!("Failed to write {}", state_path.display())
                    })?;
                }
            }

            // Track for BUILD.bazel generation
            let arch_ns_snake = to_snake_case(&arch_ns);
            let runnable_lib_label = format!(
                "//src/{}/{}:{}",
                arch_ns_snake, arch_snake, arch_snake
            );
            ns_tasks
                .entry(inst_ns_snake.clone())
                .or_default()
                .push((arch_snake.clone(), runnable_lib_label, instance.unique_name.clone(), has_params));

            generated.push(format!(
                "{}::{}",
                inst_list.namespace, instance.unique_name
            ));
        }
    }

    // Write one BUILD.bazel per namespace directory
    for (ns_snake, tasks) in &ns_tasks {
        let build_ctx = TaskBuildContext {
            tasks: tasks
                .iter()
                .map(|(arch_snake, lib_label, _, has_params)| TaskBuildEntry {
                    task_name: arch_snake.clone(),
                    runnable_lib_label: lib_label.clone(),
                    has_parameters: *has_params,
                })
                .collect(),
        };
        let build_content = render(&task_build_tmpl, &build_ctx)
            .with_context(|| format!("Failed to render task BUILD for namespace {}", ns_snake))?;
        let build_path = output_dir.join(ns_snake).join("BUILD.bazel");
        fs::write(&build_path, &build_content)
            .with_context(|| format!("Failed to write {}", build_path.display()))?;
    }

    // Write BUILD.bazel for the parameter directory
    let any_params = !fs::read_dir(&param_dir)
        .map(|entries| entries.count() == 0)
        .unwrap_or(true);
    if any_params {
        let param_build = "cc_library(\n    name = \"parameter\",\n    hdrs = glob([\"*.hpp\"]),\n    includes = [\".\"],\n    visibility = [\"//visibility:public\"],\n)\n";
        let param_build_path = param_dir.join("BUILD.bazel");
        fs::write(&param_build_path, param_build)
            .with_context(|| format!("Failed to write {}", param_build_path.display()))?;
    }

    // Write BUILD.bazel for the internal_state directory
    let any_states = !fs::read_dir(&state_dir)
        .map(|entries| entries.count() == 0)
        .unwrap_or(true);
    if any_states {
        let state_build = "cc_library(\n    name = \"internal_state\",\n    hdrs = glob([\"*.hpp\"]),\n    includes = [\".\"],\n    visibility = [\"//visibility:public\"],\n)\n";
        let state_build_path = state_dir.join("BUILD.bazel");
        fs::write(&state_build_path, state_build)
            .with_context(|| format!("Failed to write {}", state_build_path.display()))?;
    }

    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use frontend::archetypes::RunnableArchetypeList;
    use frontend::instances::InstanceList;
    use frontend::internal_state::InternalStateDefinition;
    use frontend::parameters::ParameterDefinition;

    #[test]
    fn test_generate_tasks() {
        let dir = std::env::temp_dir().join("scoregen_test_tasks");
        let _ = fs::remove_dir_all(&dir);

        let archetype_list = RunnableArchetypeList::from_str(
            r#"
namespace: ADAS
runnables:
  - unique_name: CameraController
    runnabletype: CPU
    parameter_header: params/cam.yaml
    internal_state_header: states/cam.yaml
    build_information:
      bazel_target: "//test:test"
"#,
        )
        .unwrap();

        let param_def = ParameterDefinition::from_str(
            r#"
name: CameraControllerParameters
top_level: true
members:
  - name: camera_id
    type: uint8_t
    default: 0
  - name: frame_rate_fps
    type: uint32_t
    default: 30
"#,
        )
        .unwrap();

        let mut params = HashMap::new();
        params.insert("CameraController".to_string(), param_def);

        let state_def = InternalStateDefinition::from_str(
            r#"
name: CameraState
top_level: true
members:
  - name: frame_counter
    member_type: uint64_t
    default: 0
"#,
        )
        .unwrap();

        let mut states = HashMap::new();
        states.insert("CameraController".to_string(), state_def);

        let inst_list = InstanceList::from_str(
            r#"
namespace: MiniADAS
instances:
  - archetype_reference: CameraController
    unique_name: front_camera
    parameter_overrides:
      - name: camera_id
        value: 1
      - name: frame_rate_fps
        value: 60
"#,
        )
        .unwrap();

        let generated = generate_tasks(
            &dir,
            &[inst_list],
            &[archetype_list],
            &params,
            &states,
            None,
        )
        .unwrap();

        assert_eq!(generated.len(), 1);
        assert_eq!(generated[0], "MiniADAS::front_camera");

        // Check task.cpp is at src/<name>.cpp
        let task_cpp = fs::read_to_string(
            dir.join("mini_adas/src/camera_controller.cpp"),
        )
        .unwrap();
        assert!(task_cpp.contains("int main()"));
        assert!(task_cpp.contains("const CameraControllerParameters params{};"));
        assert!(task_cpp.contains("CameraControllerInternalState state{};"));
        assert!(task_cpp.contains("adas::camera_controller::on_init(params, state)"));
        // No more inline override assignments
        assert!(!task_cpp.contains("params.camera_id ="));
        assert!(!task_cpp.contains("params.frame_rate_fps ="));
        // Include the task-level parameter header
        assert!(task_cpp.contains("#include \"camera_controller_parameter.hpp\""));
        // Include the internal state header
        assert!(task_cpp.contains("#include \"camera_controller_internal_state.hpp\""));

        // Check parameter header in src-gen/parameter/ with overrides baked in
        let param_hpp = fs::read_to_string(
            dir.join("parameter/camera_controller_parameter.hpp"),
        )
        .unwrap();
        assert!(!param_hpp.contains("namespace"));
        assert!(param_hpp.contains("struct CameraControllerParameters {"));
        assert!(param_hpp.contains("uint8_t camera_id{1}"));   // overridden
        assert!(param_hpp.contains("uint32_t frame_rate_fps{60}"));  // overridden

        // Check internal state header in src-gen/internal_state/
        let state_hpp = fs::read_to_string(
            dir.join("internal_state/camera_controller_internal_state.hpp"),
        )
        .unwrap();
        assert!(!state_hpp.contains("namespace"));
        assert!(state_hpp.contains("struct CameraControllerInternalState {"));
        assert!(state_hpp.contains("uint64_t frame_counter{0};"));

        // Check internal_state BUILD.bazel
        let state_build = fs::read_to_string(
            dir.join("internal_state/BUILD.bazel"),
        )
        .unwrap();
        assert!(state_build.contains("cc_library"));
        assert!(state_build.contains("name = \"internal_state\""));

        // Check parameter BUILD.bazel
        let param_build = fs::read_to_string(
            dir.join("parameter/BUILD.bazel"),
        )
        .unwrap();
        assert!(param_build.contains("cc_library"));
        assert!(param_build.contains("name = \"parameter\""));

        // Check single BUILD.bazel at namespace root
        let build = fs::read_to_string(
            dir.join("mini_adas/BUILD.bazel"),
        )
        .unwrap();
        // Task cc_binary
        assert!(build.contains("cc_binary"));
        assert!(build.contains("name = \"camera_controller\""));
        assert!(build.contains("\"src/camera_controller.cpp\""));
        assert!(build.contains("//src/adas/camera_controller:camera_controller"));
        // No cc_test in task BUILD (tests moved to archetype directory)
        assert!(!build.contains("cc_test"));

        let _ = fs::remove_dir_all(&dir);
    }
}

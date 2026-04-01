use serde::Serialize;

use super::{format_cpp_default, to_cpp_type, to_snake_case, MemberContext};

/// Template context for rendering task.cpp.
#[derive(Serialize)]
pub(crate) struct TaskContext {
    pub archetype_namespace: String,
    pub task_namespace: String,
    pub class_name: String,
    pub runnable_snake_name: String,
    pub task_name: String,
    pub has_parameters: bool,
    pub has_internal_state: bool,
    pub has_inputs: bool,
    pub has_outputs: bool,
    pub parameters_struct_name: String,
    pub internal_state_struct_name: String,
}

/// A single task entry for the BUILD template.
#[derive(Serialize)]
pub(crate) struct TaskBuildEntry {
    pub task_name: String,
    pub runnable_lib_label: String,
    pub has_parameters: bool,
}

/// Template context for rendering a single BUILD.bazel with all task binaries.
#[derive(Serialize)]
pub(crate) struct TaskBuildContext {
    pub tasks: Vec<TaskBuildEntry>,
}

/// Template context for rendering a task-level parameter header.
/// No namespace — parameters are in global scope.
#[derive(Serialize)]
pub(crate) struct TaskParameterContext {
    pub struct_name: String,
    pub guard: String,
    pub members: Vec<MemberContext>,
}

impl TaskParameterContext {
    /// Build from archetype parameters with instance overrides merged in.
    /// Parameters are in global scope — no namespace.
    pub fn new(
        archetype_name: &str,
        param_def: &frontend::parameters::ParameterDefinition,
        overrides: &[frontend::instances::ParameterOverride],
    ) -> Self {
        let snake_name = to_snake_case(archetype_name);
        let struct_name = format!("{}Parameters", archetype_name);
        let guard = format!(
            "{}_PARAMETER_HPP",
            snake_name.to_uppercase(),
        );
        let members = param_def
            .members
            .iter()
            .map(|m| {
                // Check if this member has an instance override
                let overridden = overrides.iter().find(|o| o.name == m.name);
                let default_value = if let Some(ov) = overridden {
                    Some(format_cpp_default(&ov.value, &m.member_type))
                } else {
                    m.default.as_ref().map(|v| format_cpp_default(v, &m.member_type))
                };
                MemberContext {
                    name: m.name.clone(),
                    member_type: to_cpp_type(&m.member_type),
                    default_value,
                }
            })
            .collect();
        Self {
            struct_name,
            guard,
            members,
        }
    }
}

impl TaskContext {
    /// Build a TaskContext from instance data.
    pub fn new(
        archetype_ns: &str,
        instance_ns: &str,
        archetype_name: &str,
        has_parameters: bool,
        has_internal_state: bool,
        has_inputs: bool,
        has_outputs: bool,
    ) -> Self {
        let snake_name = to_snake_case(archetype_name);
        let params_struct = format!("{}Parameters", archetype_name);
        let state_struct = format!("{}InternalState", archetype_name);
        Self {
            archetype_namespace: to_snake_case(archetype_ns),
            task_namespace: to_snake_case(instance_ns),
            class_name: archetype_name.to_string(),
            runnable_snake_name: snake_name.clone(),
            task_name: snake_name,
            has_parameters,
            has_internal_state,
            has_inputs,
            has_outputs,
            parameters_struct_name: params_struct,
            internal_state_struct_name: state_struct,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::render;

    const TASK_TEMPLATE: &str = include_str!("../../templates/task.cpp.j2");
    const TASK_BUILD_TEMPLATE: &str = include_str!("../../templates/task_BUILD.bazel.j2");

    #[test]
    fn test_render_task_with_params_and_state() {
        let ctx = TaskContext {
            archetype_namespace: "adas".to_string(),
            task_namespace: "mini_adas".to_string(),
            class_name: "CameraController".to_string(),
            runnable_snake_name: "camera_controller".to_string(),
            task_name: "camera_controller".to_string(),
            has_parameters: true,
            has_internal_state: true,
            has_inputs: false,
            has_outputs: false,
            parameters_struct_name: "CameraControllerParameters".to_string(),
            internal_state_struct_name: "CameraControllerInternalState".to_string(),
        };
        let output = render(TASK_TEMPLATE, &ctx).unwrap();
        assert!(output.contains("#include \"camera_controller.hpp\""));
        assert!(output.contains("#include \"camera_controller_parameter.hpp\""));
        assert!(output.contains("#include \"camera_controller_internal_state.hpp\""));
        assert!(output.contains("int main()"));
        assert!(output.contains("const CameraControllerParameters params{};"));
        assert!(output.contains("CameraControllerInternalState state{};"));
        assert!(output.contains("adas::camera_controller::on_init(params, state)"));
        assert!(output.contains("adas::camera_controller::on_update(params, state)"));
    }

    #[test]
    fn test_render_task_no_params_no_state() {
        let ctx = TaskContext {
            archetype_namespace: "test_ns".to_string(),
            task_namespace: "task_ns".to_string(),
            class_name: "Simple".to_string(),
            runnable_snake_name: "simple".to_string(),
            task_name: "simple".to_string(),
            has_parameters: false,
            has_internal_state: false,
            has_inputs: false,
            has_outputs: false,
            parameters_struct_name: "SimpleParameters".to_string(),
            internal_state_struct_name: "SimpleInternalState".to_string(),
        };
        let output = render(TASK_TEMPLATE, &ctx).unwrap();
        assert!(output.contains("int main()"));
        assert!(output.contains("on_init()"));
        assert!(output.contains("on_update()"));
        assert!(!output.contains("Parameters"));
        assert!(!output.contains("InternalState"));
    }

    #[test]
    fn test_render_task_params_only() {
        let ctx = TaskContext {
            archetype_namespace: "ns".to_string(),
            task_namespace: "task_ns".to_string(),
            class_name: "Foo".to_string(),
            runnable_snake_name: "foo".to_string(),
            task_name: "foo".to_string(),
            has_parameters: true,
            has_internal_state: false,
            has_inputs: false,
            has_outputs: false,
            parameters_struct_name: "FooParameters".to_string(),
            internal_state_struct_name: "FooInternalState".to_string(),
        };
        let output = render(TASK_TEMPLATE, &ctx).unwrap();
        assert!(output.contains("const FooParameters params{};"));
        assert!(output.contains("on_init(params)"));
        assert!(output.contains("on_update(params)"));
        assert!(!output.contains("InternalState"));
    }

    #[test]
    fn test_render_task_test_with_params_and_state() {
        // Unit test template now uses RunnableContext, test in runnable.rs
    }

    #[test]
    fn test_render_task_build() {
        let ctx = TaskBuildContext {
            tasks: vec![
                TaskBuildEntry {
                    task_name: "camera_controller".to_string(),
                    runnable_lib_label: "//src/adas/camera_controller:camera_controller"
                        .to_string(),
                    has_parameters: true,
                },
                TaskBuildEntry {
                    task_name: "brake_controller".to_string(),
                    runnable_lib_label: "//src/adas/brake_controller:brake_controller"
                        .to_string(),
                    has_parameters: false,
                },
            ],
        };
        let output = render(TASK_BUILD_TEMPLATE, &ctx).unwrap();
        // Task cc_binary
        assert!(output.contains("name = \"camera_controller\""));
        assert!(output.contains("//src/adas/camera_controller:camera_controller"));
        assert!(output.contains("name = \"brake_controller\""));
        assert!(output.contains("//src/adas/brake_controller:brake_controller"));
        assert_eq!(output.matches("cc_binary").count(), 2);
        // No cc_test in task BUILD (tests moved to archetype)
        assert_eq!(output.matches("cc_test").count(), 0);
    }

    #[test]
    fn test_task_parameter_context_with_overrides() {
        use frontend::parameters::ParameterDefinition;
        use frontend::instances::ParameterOverride;

        let param_def = ParameterDefinition::from_str(
            r#"
name: CameraControllerParameters
top_level: true
members:
  - name: camera_id
    member_type: uint8_t
    default: 0
  - name: frame_rate_fps
    member_type: uint32_t
    default: 30
  - name: resolution_width
    member_type: uint32_t
    default: 640
"#,
        )
        .unwrap();
        let overrides = vec![
            ParameterOverride {
                name: "resolution_width".to_string(),
                value: serde_yaml::Value::Number(serde_yaml::Number::from(1920)),
            },
        ];

        let ctx = TaskParameterContext::new("CameraController", &param_def, &overrides);

        assert_eq!(ctx.struct_name, "CameraControllerParameters");
        assert_eq!(ctx.guard, "CAMERA_CONTROLLER_PARAMETER_HPP");
        assert_eq!(ctx.members.len(), 3);
        // Non-overridden: keep archetype default
        assert_eq!(ctx.members[0].default_value, Some("0".to_string()));
        assert_eq!(ctx.members[1].default_value, Some("30".to_string()));
        // Overridden: use instance value
        assert_eq!(ctx.members[2].default_value, Some("1920".to_string()));

        // Render with the parameters template
        const PARAMS_TEMPLATE: &str = include_str!("../../templates/parameters.hpp.j2");
        let output = render(PARAMS_TEMPLATE, &ctx).unwrap();
        assert!(output.contains("struct CameraControllerParameters {"));
        assert!(!output.contains("namespace"));
        assert!(output.contains("uint32_t resolution_width{1920}"));
        assert!(output.contains("CAMERA_CONTROLLER_PARAMETER_HPP"));
    }
}

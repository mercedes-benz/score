use serde::Serialize;

use super::MemberContext;

/// Template context for generating the parameters header file.
#[derive(Serialize)]
pub(crate) struct ParameterContext {
    pub struct_name: String,
    pub guard: String,
    pub members: Vec<MemberContext>,
}

#[cfg(test)]
impl ParameterContext {
    pub fn from(
        _namespace: &str,
        runnable: &frontend::archetypes::RunnableArchetypeDescription,
        param_def: &frontend::parameters::ParameterDefinition,
    ) -> Self {
        use super::{format_cpp_default, to_cpp_type, to_snake_case};
        let snake = to_snake_case(&runnable.unique_name);
        let struct_name = format!("{}Parameters", runnable.unique_name);
        let guard = format!(
            "{}_PARAMETER_HPP",
            snake.to_uppercase(),
        );
        let members = param_def
            .members
            .iter()
            .map(|m| MemberContext {
                name: m.name.clone(),
                member_type: to_cpp_type(&m.member_type),
                default_value: m.default.as_ref().map(|v| format_cpp_default(v, &m.member_type)),
            })
            .collect();
        Self {
            struct_name,
            guard,
            members,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::{render, DEFAULT_PARAMS_TEMPLATE};
    use frontend::archetypes::RunnableArchetypeDescription;
    use frontend::parameters::ParameterDefinition;

    #[test]
    fn test_render_parameters_template() {
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
  - name: exposure_auto
    member_type: bool
    default: true
  - name: gain
    member_type: float
    default: 1.0
"#,
        )
        .unwrap();
        let runnable = RunnableArchetypeDescription {
            unique_name: "CameraController".to_string(),
            ..Default::default()
        };
        let ctx = ParameterContext::from("ADAS", &runnable, &param_def);
        let output = render(DEFAULT_PARAMS_TEMPLATE, &ctx).unwrap();
        assert!(output.contains("struct CameraControllerParameters {"));
        assert!(!output.contains("namespace"));
        assert!(output.contains("uint8_t camera_id{0};"));
        assert!(output.contains("uint32_t frame_rate_fps{30};"));
        assert!(output.contains("bool exposure_auto{true};"));
        assert!(output.contains("float gain{1.0f};"));
    }
}

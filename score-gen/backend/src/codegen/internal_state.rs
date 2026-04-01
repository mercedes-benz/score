use frontend::archetypes::RunnableArchetypeDescription;
use frontend::internal_state::InternalStateDefinition;
use serde::Serialize;

use super::{format_cpp_default, to_cpp_type, to_snake_case, MemberContext};

/// Template context for generating the internal state header file.
#[derive(Serialize)]
pub(crate) struct InternalStateContext {
    pub struct_name: String,
    pub guard: String,
    pub members: Vec<MemberContext>,
}

impl InternalStateContext {
    pub fn from(
        _namespace: &str,
        runnable: &RunnableArchetypeDescription,
        state_def: &InternalStateDefinition,
    ) -> Self {
        let snake = to_snake_case(&runnable.unique_name);
        let struct_name = format!("{}InternalState", runnable.unique_name);
        let guard = format!(
            "{}_INTERNAL_STATE_HPP",
            snake.to_uppercase(),
        );
        let members = state_def
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
    use crate::codegen::{render, DEFAULT_STATE_TEMPLATE};
    use frontend::archetypes::RunnableArchetypeDescription;
    use frontend::internal_state::InternalStateDefinition;

    #[test]
    fn test_render_internal_state_template() {
        let state_def = InternalStateDefinition::from_str(
            r#"
name: CameraState
top_level: true
members:
  - name: frame_counter
    member_type: uint64_t
    default: 0
  - name: is_streaming
    member_type: bool
    default: false
  - name: buffer
    member_type: float[1024]
"#,
        )
        .unwrap();
        let runnable = RunnableArchetypeDescription {
            unique_name: "CameraController".to_string(),
            ..Default::default()
        };
        let ctx = InternalStateContext::from("ADAS", &runnable, &state_def);
        let output = render(DEFAULT_STATE_TEMPLATE, &ctx).unwrap();
        assert!(output.contains("struct CameraControllerInternalState {"));
        assert!(!output.contains("namespace"));
        assert!(output.contains("uint64_t frame_counter{0};"));
        assert!(output.contains("bool is_streaming{false};"));
        assert!(output.contains("std::array<float, 1024> buffer;"));
    }
}

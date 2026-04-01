use frontend::archetypes::{RunnableArchetypeDescription, RunnableArchetypeList};
use serde::Serialize;
use std::collections::HashMap;

use super::to_snake_case;

/// Build a map of `type_name → max consumer n_samples_max` from all inputs
/// across all runnables. Used to compute default n_slots for outputs.
pub(crate) fn build_consumer_map(archetype_lists: &[RunnableArchetypeList]) -> HashMap<String, u16> {
    let mut map: HashMap<String, u16> = HashMap::new();
    for list in archetype_lists {
        for runnable in &list.runnables {
            for input in &runnable.inputs {
                let samples = if input.n_samples_max == 0 { 1 } else { input.n_samples_max };
                let entry = map.entry(input.type_name.clone()).or_insert(0);
                if samples > *entry {
                    *entry = samples;
                }
            }
        }
    }
    map
}

/// Describes a single input queue within the generated Inputs struct.
#[derive(Serialize, Clone, Debug)]
pub(crate) struct InputQueueContext {
    /// Variable name (from archetype input `unique_name`).
    pub variable_name: String,
    /// Fully-qualified C++ type (e.g. `adas::perception::CameraFrame`).
    pub type_name: String,
    /// Maximum queue capacity (from `n_samples_max`, defaults to 1).
    pub capacity: u16,
}

/// Describes a single output slot within the generated Outputs struct.
#[derive(Serialize, Clone, Debug)]
pub(crate) struct OutputQueueContext {
    /// Variable name (from archetype output `unique_name`).
    pub variable_name: String,
    /// Fully-qualified C++ type (e.g. `adas::perception::CameraFrame`).
    pub type_name: String,
    /// Maximum samples capacity (from `n_samples_max`, defaults to 1).
    pub capacity: u16,
    /// Number of slots (producer + consumers).
    pub n_slots: u16,
}

/// Template context passed to MiniJinja for rendering runnable files.
#[derive(Serialize)]
pub(crate) struct RunnableContext {
    pub namespace: String,
    pub class_name: String,
    pub snake_name: String,
    pub guard: String,
    pub runnabletype: String,
    pub has_parameters: bool,
    pub has_internal_state: bool,
    pub parameters_struct_name: String,
    pub internal_state_struct_name: String,
    pub has_inputs: bool,
    pub inputs: Vec<InputQueueContext>,
    pub has_outputs: bool,
    pub outputs: Vec<OutputQueueContext>,
    /// Include paths for input/output types (e.g. `"adas/perception/camera_frame.hpp"`).
    pub io_includes: Vec<String>,
}

impl RunnableContext {
    /// Create a RunnableContext from an archetype description.
    ///
    /// `consumer_map` maps output `type_name` → max `n_samples_max` across all
    /// consuming inputs. When `n_slots` is not specified on an output, it is
    /// computed as: `max_consumer_samples * 2 + producer_n_samples_max`.
    /// Pass `None` when consumer information is unavailable (defaults to 1).
    pub fn from(
        namespace: &str,
        runnable: &RunnableArchetypeDescription,
        consumer_map: Option<&HashMap<String, u16>>,
    ) -> Self {
        let snake_ns = to_snake_case(namespace);
        let snake = to_snake_case(&runnable.unique_name);
        let guard = format!(
            "{}_{}_{}_HPP",
            snake_ns.to_uppercase(),
            snake.to_uppercase(),
            "GENERATED"
        );
        let params_struct = format!("{}Parameters", runnable.unique_name);
        let state_struct = format!("{}InternalState", runnable.unique_name);

        let inputs: Vec<InputQueueContext> = runnable
            .inputs
            .iter()
            .map(|inp| {
                let capacity = if inp.n_samples_max == 0 { 1 } else { inp.n_samples_max };
                InputQueueContext {
                    variable_name: inp.unique_name.clone(),
                    type_name: inp.type_name.clone(),
                    capacity,
                }
            })
            .collect();

        let outputs: Vec<OutputQueueContext> = runnable
            .outputs
            .iter()
            .map(|out| {
                let capacity = if out.n_samples_max == 0 { 1 } else { out.n_samples_max };
                let n_slots = out.n_slots.unwrap_or_else(|| {
                    // max_consumer_samples * 2 + producer_n_samples_max
                    let max_consumer = consumer_map
                        .and_then(|m| m.get(&out.type_name).copied())
                        .unwrap_or(0);
                    max_consumer * 2 + capacity
                });
                OutputQueueContext {
                    variable_name: out.unique_name.clone(),
                    type_name: out.type_name.clone(),
                    capacity,
                    n_slots,
                }
            })
            .collect();

        // Build unique include paths from input and output type_names.
        // "adas::perception::CameraFrame" → "adas/perception/camera_frame.hpp"
        let type_name_to_include = |type_name: &str| -> String {
            let parts: Vec<&str> = type_name.rsplitn(2, "::").collect();
            let struct_name = parts[0];
            let ns_path = if parts.len() > 1 {
                parts[1].replace("::", "/")
            } else {
                String::new()
            };
            if ns_path.is_empty() {
                format!("{}.hpp", to_snake_case(struct_name))
            } else {
                format!("{}/{}.hpp", ns_path, to_snake_case(struct_name))
            }
        };
        let mut io_includes: Vec<String> = inputs
            .iter()
            .map(|inp| type_name_to_include(&inp.type_name))
            .chain(outputs.iter().map(|out| type_name_to_include(&out.type_name)))
            .collect();
        io_includes.sort();
        io_includes.dedup();

        Self {
            namespace: snake_ns,
            class_name: runnable.unique_name.clone(),
            snake_name: snake,
            guard,
            runnabletype: runnable.runnabletype.clone(),
            has_parameters: runnable.parameter_header.is_some(),
            has_internal_state: runnable.internal_state_header.is_some(),
            parameters_struct_name: params_struct,
            internal_state_struct_name: state_struct,
            has_inputs: !inputs.is_empty(),
            has_outputs: !outputs.is_empty(),
            io_includes,
            inputs,
            outputs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::render;
    use super::super::generate::load_templates;
    use frontend::archetypes::{RunnableArchetypeDescription, RunnableArchetypeList};

    #[test]
    fn test_render_header_with_default_templates() {
        let templates = load_templates(None).unwrap();
        let runnable = RunnableArchetypeDescription {
            unique_name: "CameraController".to_string(),
            ..Default::default()
        };
        let ctx = RunnableContext::from("ADAS", &runnable, None);
        let header = render(&templates.header, &ctx).unwrap();
        assert!(header.contains("namespace camera_controller"));
        assert!(header.contains("score::base::ErrorCode on_init()"));
        assert!(header.contains("namespace adas"));
        assert!(header.contains("#include \"score/base/error.hpp\""));
    }

    #[test]
    fn test_render_source_with_default_templates() {
        let templates = load_templates(None).unwrap();
        let runnable = RunnableArchetypeDescription {
            unique_name: "CameraController".to_string(),
            ..Default::default()
        };
        let ctx = RunnableContext::from("ADAS", &runnable, None);
        let source = render(&templates.source, &ctx).unwrap();
        assert!(source.contains("score::base::ErrorCode on_init()"));
        assert!(source.contains("score::base::ErrorCode::Success()"));
        assert!(source.contains("#include \"camera_controller.hpp\""));
        assert!(source.contains("namespace adas"));
        assert!(source.contains("namespace camera_controller"));
    }

    #[test]
    fn test_render_header_with_parameters() {
        let templates = load_templates(None).unwrap();
        let runnable = RunnableArchetypeDescription {
            unique_name: "CameraController".to_string(),
            parameter_header: Some("parameters/CameraControllerParameters.yaml".to_string()),
            ..Default::default()
        };
        let ctx = RunnableContext::from("ADAS", &runnable, None);
        let header = render(&templates.header, &ctx).unwrap();
        assert!(header.contains("struct CameraControllerParameters;"));
        assert!(!header.contains("#include \"parameters.hpp\""));
        assert!(header.contains("on_init(const CameraControllerParameters& parameters)"));
        assert!(header.contains("on_update(const CameraControllerParameters& parameters)"));
    }

    #[test]
    fn test_render_source_with_parameters() {
        let templates = load_templates(None).unwrap();
        let runnable = RunnableArchetypeDescription {
            unique_name: "CameraController".to_string(),
            parameter_header: Some("parameters/CameraControllerParameters.yaml".to_string()),
            ..Default::default()
        };
        let ctx = RunnableContext::from("ADAS", &runnable, None);
        let source = render(&templates.source, &ctx).unwrap();
        assert!(source.contains("on_init(const CameraControllerParameters& parameters)"));
        assert!(source.contains("on_update(const CameraControllerParameters& parameters)"));
    }

    #[test]
    fn test_render_header_with_internal_state() {
        let templates = load_templates(None).unwrap();
        let runnable = RunnableArchetypeDescription {
            unique_name: "CameraController".to_string(),
            internal_state_header: Some("internal_states/CameraState.yaml".to_string()),
            ..Default::default()
        };
        let ctx = RunnableContext::from("ADAS", &runnable, None);
        let header = render(&templates.header, &ctx).unwrap();
        assert!(header.contains("struct CameraControllerInternalState;"));
        assert!(!header.contains("#include \"internal_state.hpp\""));
        assert!(header.contains("on_init(CameraControllerInternalState& state)"));
        assert!(header.contains("on_update(CameraControllerInternalState& state)"));
        assert!(!header.contains("CameraControllerParameters"));
    }

    #[test]
    fn test_render_header_with_params_and_state() {
        let templates = load_templates(None).unwrap();
        let runnable = RunnableArchetypeDescription {
            unique_name: "CameraController".to_string(),
            parameter_header: Some("parameters/CameraControllerParameters.yaml".to_string()),
            internal_state_header: Some("internal_states/CameraState.yaml".to_string()),
            ..Default::default()
        };
        let ctx = RunnableContext::from("ADAS", &runnable, None);
        let header = render(&templates.header, &ctx).unwrap();
        assert!(header.contains("struct CameraControllerParameters;"));
        assert!(header.contains("struct CameraControllerInternalState;"));
        assert!(header.contains("on_init(const CameraControllerParameters& parameters, CameraControllerInternalState& state)"));
        assert!(header.contains("on_update(const CameraControllerParameters& parameters, CameraControllerInternalState& state)"));
    }

    #[test]
    fn test_render_source_with_internal_state() {
        let templates = load_templates(None).unwrap();
        let runnable = RunnableArchetypeDescription {
            unique_name: "CameraController".to_string(),
            internal_state_header: Some("internal_states/CameraState.yaml".to_string()),
            ..Default::default()
        };
        let ctx = RunnableContext::from("ADAS", &runnable, None);
        let source = render(&templates.source, &ctx).unwrap();
        assert!(source.contains("on_init(CameraControllerInternalState& state)"));
        assert!(source.contains("on_update(CameraControllerInternalState& state)"));
    }

    #[test]
    fn test_render_unit_test_with_params_and_state() {
        let templates = load_templates(None).unwrap();
        let runnable = RunnableArchetypeDescription {
            unique_name: "CameraController".to_string(),
            parameter_header: Some("parameters/CameraControllerParameters.yaml".to_string()),
            internal_state_header: Some("internal_states/CameraState.yaml".to_string()),
            ..Default::default()
        };
        let ctx = RunnableContext::from("ADAS", &runnable, None);
        let output = render(&templates.unit_test, &ctx).unwrap();
        assert!(output.contains("#include <gtest/gtest.h>"));
        assert!(output.contains("class CameraControllerTest : public ::testing::Test"));
        assert!(output.contains("const CameraControllerParameters params_{};"));
        assert!(output.contains("CameraControllerInternalState state_{};"));
        assert!(output.contains("TEST_F(CameraControllerTest, OnInitReturnsSuccess)"));
        assert!(output.contains("TEST_F(CameraControllerTest, OnUpdateReturnsSuccess)"));
        assert!(output.contains("adas::camera_controller::on_init(params_, state_)"));
        assert!(output.contains("adas::camera_controller::on_update(params_, state_)"));
    }

    #[test]
    fn test_render_header_with_inputs() {
        use frontend::archetypes::RunnableInputDescription;
        let templates = load_templates(None).unwrap();
        let runnable = RunnableArchetypeDescription {
            unique_name: "NeuralNetInference".to_string(),
            parameter_header: Some("parameters/NeuralNetParameters.yaml".to_string()),
            internal_state_header: Some("internal_states/NeuralNetState.yaml".to_string()),
            inputs: vec![
                RunnableInputDescription {
                    unique_name: "input_frames".to_string(),
                    type_name: "adas::perception::CameraFrame".to_string(),
                    n_samples_max: 2,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let ctx = RunnableContext::from("ADAS", &runnable, None);
        let header = render(&templates.header, &ctx).unwrap();
        // Check includes
        assert!(header.contains("#include <deque>"));
        assert!(header.contains("#include \"adas/perception/camera_frame.hpp\""));
        // Check Inputs struct
        assert!(header.contains("struct Inputs {"));
        assert!(header.contains("static constexpr std::size_t input_frames_capacity = 2;"));
        assert!(header.contains("std::deque<const adas::perception::CameraFrame*> input_frames{input_frames_capacity, nullptr};"));        // Check on_update signature: const refs first (parameters, inputs), then non-const (state)
        assert!(header.contains("on_update(const NeuralNetInferenceParameters& parameters, const Inputs& inputs, NeuralNetInferenceInternalState& state)"));
        // on_init should NOT have inputs
        assert!(!header.contains("on_init") || !header.lines().any(|l| l.contains("on_init") && l.contains("Inputs")));
    }

    #[test]
    fn test_render_source_with_inputs() {
        use frontend::archetypes::RunnableInputDescription;
        let templates = load_templates(None).unwrap();
        let runnable = RunnableArchetypeDescription {
            unique_name: "BrakeController".to_string(),
            parameter_header: Some("parameters/BrakeControllerParameters.yaml".to_string()),
            internal_state_header: Some("internal_states/BrakeControllerState.yaml".to_string()),
            inputs: vec![
                RunnableInputDescription {
                    unique_name: "object_detections".to_string(),
                    type_name: "adas::perception::ObjectDetections".to_string(),
                    n_samples_max: 1,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let ctx = RunnableContext::from("ADAS", &runnable, None);
        let source = render(&templates.source, &ctx).unwrap();
        assert!(source.contains("on_update(const BrakeControllerParameters& parameters, const Inputs& inputs, BrakeControllerInternalState& state)"));
    }

    #[test]
    fn test_render_unit_test_with_inputs() {
        use frontend::archetypes::RunnableInputDescription;
        let templates = load_templates(None).unwrap();
        let runnable = RunnableArchetypeDescription {
            unique_name: "BrakeController".to_string(),
            parameter_header: Some("parameters/BrakeControllerParameters.yaml".to_string()),
            internal_state_header: Some("internal_states/BrakeControllerState.yaml".to_string()),
            inputs: vec![
                RunnableInputDescription {
                    unique_name: "object_detections".to_string(),
                    type_name: "adas::perception::ObjectDetections".to_string(),
                    n_samples_max: 1,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let ctx = RunnableContext::from("ADAS", &runnable, None);
        let output = render(&templates.unit_test, &ctx).unwrap();
        assert!(output.contains("const adas::brake_controller::Inputs inputs_{};"));
        assert!(output.contains("on_update(params_, inputs_, state_)"));
    }

    #[test]
    fn test_input_default_capacity() {
        use frontend::archetypes::RunnableInputDescription;
        let runnable = RunnableArchetypeDescription {
            unique_name: "Test".to_string(),
            inputs: vec![
                RunnableInputDescription {
                    unique_name: "data".to_string(),
                    type_name: "ns::Type".to_string(),
                    n_samples_max: 0,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let ctx = RunnableContext::from("NS", &runnable, None);
        // n_samples_max=0 should default to capacity 1
        assert_eq!(ctx.inputs[0].capacity, 1);
    }

    #[test]
    fn test_default_n_slots_computation() {
        use frontend::archetypes::RunnableOutputDescription;

        let lists = vec![RunnableArchetypeList::from_str(
            r#"
namespace: Test
runnables:
  - unique_name: Camera
    runnabletype: CPU
    outputs:
      - unique_name: frames
        type_name: "ns::CameraFrame"
        n_samples_max: 1
    build_information:
      bazel_target: "//a:a"
  - unique_name: DetectorA
    runnabletype: CPU
    inputs:
      - unique_name: input
        type_name: "ns::CameraFrame"
        n_samples_max: 2
    build_information:
      bazel_target: "//b:b"
  - unique_name: Logger
    runnabletype: CPU
    inputs:
      - unique_name: log
        type_name: "ns::CameraFrame"
        n_samples_max: 5
    build_information:
      bazel_target: "//c:c"
"#,
        ).unwrap()];

        let consumer_map = build_consumer_map(&lists);
        // max consumer n_samples_max for ns::CameraFrame is 5
        assert_eq!(consumer_map.get("ns::CameraFrame"), Some(&5));

        // Producer with no explicit n_slots
        let producer = &lists[0].runnables[0];
        let ctx = RunnableContext::from("Test", producer, Some(&consumer_map));
        // default n_slots = max_consumer(5) * 2 + producer_n_samples_max(1) = 11
        assert_eq!(ctx.outputs[0].n_slots, 11);

        // Explicit n_slots should override the default
        let producer_explicit = RunnableArchetypeDescription {
            unique_name: "Camera".to_string(),
            outputs: vec![RunnableOutputDescription {
                unique_name: "frames".to_string(),
                type_name: "ns::CameraFrame".to_string(),
                n_samples_max: 1,
                n_slots: Some(3),
                ..Default::default()
            }],
            ..Default::default()
        };
        let ctx2 = RunnableContext::from("Test", &producer_explicit, Some(&consumer_map));
        assert_eq!(ctx2.outputs[0].n_slots, 3);
    }
}

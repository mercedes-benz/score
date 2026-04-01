use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;

/// A parameter override for an instance.
#[derive(Deserialize, Clone, PartialEq, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParameterOverride {
    pub name: String,
    #[schemars(with = "serde_json::Value")]
    pub value: serde_yaml::Value,
}

/// An input binding for an instance.
#[derive(Deserialize, Clone, PartialEq, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InputBinding {
    pub input_reference: String,
    pub topic_name: String,
}

/// An output binding for an instance.
#[derive(Deserialize, Clone, PartialEq, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OutputBinding {
    pub output_reference: String,
    pub topic_name: String,
}

/// Execution parameters for an instance.
#[derive(Deserialize, Clone, PartialEq, Debug, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionParams {
    #[serde(default)]
    pub autostart: bool,
    #[serde(default)]
    pub autorestart: bool,
}

/// A single runnable instance.
#[derive(Deserialize, Clone, PartialEq, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Instance {
    pub archetype_reference: String,
    pub unique_name: String,
    #[serde(default)]
    pub parameter_overrides: Vec<ParameterOverride>,
    #[serde(default)]
    pub inputs: Vec<InputBinding>,
    #[serde(default)]
    pub outputs: Vec<OutputBinding>,
    #[serde(default)]
    pub execution_params: ExecutionParams,
}

/// A list of runnable instances within a namespace.
#[derive(Deserialize, Clone, PartialEq, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstanceList {
    pub namespace: String,
    pub instances: Vec<Instance>,
}

impl InstanceList {
    /// Parse an `InstanceList` from a YAML file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let list = Self::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(list)
    }

    /// Parse an `InstanceList` from a YAML string.
    pub fn from_str(yaml: &str) -> Result<Self> {
        let list: InstanceList = serde_yaml::from_str(yaml)?;
        list.validate()?;
        Ok(list)
    }

    fn validate(&self) -> Result<()> {
        if self.namespace.is_empty() {
            anyhow::bail!("Instance namespace must not be empty");
        }
        let mut unique_names = HashSet::new();
        for inst in &self.instances {
            if !unique_names.insert(&inst.unique_name) {
                anyhow::bail!(
                    "Duplicate instance unique_name: {}",
                    inst.unique_name
                );
            }
            if inst.archetype_reference.is_empty() {
                anyhow::bail!(
                    "archetype_reference must not be empty for instance '{}'",
                    inst.unique_name
                );
            }
        }
        Ok(())
    }

    /// Find an instance by unique_name.
    pub fn find_instance(&self, name: &str) -> Option<&Instance> {
        self.instances.iter().find(|i| i.unique_name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_YAML: &str = r#"
namespace: MiniADAS

instances:
  - archetype_reference: CameraController
    unique_name: front_camera
    parameter_overrides:
      - name: camera_id
        value: 0
      - name: frame_rate_fps
        value: 30
    outputs:
      - output_reference: camera_frames
        topic_name: MiniADAS:CameraSensor.front_camera:raw_frames
    execution_params:
      autostart: true
      autorestart: true

  - archetype_reference: BrakeController
    unique_name: emergency_brake_ctrl
    parameter_overrides:
      - name: ttc_threshold_seconds
        value: 2.5
    inputs:
      - input_reference: object_detections
        topic_name: MiniADAS:ObjectDetection.object_detector:detected_objects
    outputs:
      - output_reference: brake_commands
        topic_name: MiniADAS:BrakeControl.emergency_brake_ctrl:brake_commands
    execution_params:
      autostart: true
      autorestart: true
"#;

    #[test]
    fn test_parse_instance_list() {
        let list = InstanceList::from_str(SAMPLE_YAML).unwrap();
        assert_eq!(list.namespace, "MiniADAS");
        assert_eq!(list.instances.len(), 2);
    }

    #[test]
    fn test_instance_fields() {
        let list = InstanceList::from_str(SAMPLE_YAML).unwrap();
        let cam = list.find_instance("front_camera").unwrap();
        assert_eq!(cam.archetype_reference, "CameraController");
        assert_eq!(cam.parameter_overrides.len(), 2);
        assert_eq!(cam.parameter_overrides[0].name, "camera_id");
        assert_eq!(cam.outputs.len(), 1);
        assert!(cam.execution_params.autostart);
    }

    #[test]
    fn test_find_instance() {
        let list = InstanceList::from_str(SAMPLE_YAML).unwrap();
        assert!(list.find_instance("front_camera").is_some());
        assert!(list.find_instance("emergency_brake_ctrl").is_some());
        assert!(list.find_instance("nonexistent").is_none());
    }

    #[test]
    fn test_reject_empty_namespace() {
        let yaml = r#"
namespace: ""
instances: []
"#;
        assert!(InstanceList::from_str(yaml).is_err());
    }

    #[test]
    fn test_reject_duplicate_unique_names() {
        let yaml = r#"
namespace: Test
instances:
  - archetype_reference: Foo
    unique_name: dup
  - archetype_reference: Bar
    unique_name: dup
"#;
        assert!(InstanceList::from_str(yaml).is_err());
    }

    #[test]
    fn test_reject_empty_archetype_reference() {
        let yaml = r#"
namespace: Test
instances:
  - archetype_reference: ""
    unique_name: bad
"#;
        assert!(InstanceList::from_str(yaml).is_err());
    }

    #[test]
    fn test_parse_test_example_file() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../score-examples/test-example/graph_yaml/instances.yaml");
        if path.exists() {
            let list = InstanceList::from_file(&path).unwrap();
            assert_eq!(list.namespace, "MiniADAS");
            assert_eq!(list.instances.len(), 6);
            assert!(list.find_instance("front_camera").is_some());
            assert!(list.find_instance("object_detector").is_some());
            assert!(list.find_instance("traffic_sign_controller").is_some());
            assert!(list.find_instance("emergency_brake_ctrl").is_some());
            assert!(list.find_instance("system_data_logger").is_some());
            assert!(list.find_instance("system_perf_monitor").is_some());
        }
    }
}

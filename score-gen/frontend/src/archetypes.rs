use anyhow::Result;
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;
use std::path::Path;

/// Batch processing policy for input samples
#[derive(Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub enum BatchPolicy {
    /// Flush the input queue on every `onUpdate()` trigger
    #[serde(alias = "kLastN")]
    LastN,
    /// Retain the newest N samples across triggers
    #[serde(alias = "kNewestN")]
    NewestN,
}

impl Default for BatchPolicy {
    fn default() -> Self {
        BatchPolicy::LastN
    }
}

/// Memory backend for zero-copy communication
#[derive(Deserialize, Clone, Copy, PartialEq, Debug, JsonSchema)]
pub enum MemoryBackend {
    /// POSIX shared memory
    #[serde(alias = "kPosixShm")]
    PosixShm,
}

impl Default for MemoryBackend {
    fn default() -> Self {
        MemoryBackend::PosixShm
    }
}

/// Description of a runnable's input interface
#[derive(Deserialize, Clone, PartialEq, Debug, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunnableInputDescription {
    pub unique_name: String,
    pub type_name: String,
    pub n_samples_max: u16,
    #[serde(default)]
    pub n_samples_min: u16,
    #[serde(default)]
    pub policy: BatchPolicy,
}

/// Description of a runnable's output interface
#[derive(Deserialize, Clone, PartialEq, Debug, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunnableOutputDescription {
    pub unique_name: String,
    pub type_name: String,
    pub n_samples_max: u16,
    #[serde(default)]
    pub n_slots: Option<u16>,
    #[serde(default)]
    pub memory_backend: MemoryBackend,
}

/// Build information for a runnable archetype
#[derive(Deserialize, Clone, PartialEq, Debug, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildInformation {
    pub bazel_target: String,
}

/// A runnable archetype description
#[derive(Deserialize, Clone, PartialEq, Debug, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunnableArchetypeDescription {
    pub unique_name: String,
    pub runnabletype: String,
    #[serde(default)]
    pub wcet_us: Option<i64>,
    #[serde(default)]
    pub min_interval_us: Option<i64>,
    #[serde(default)]
    pub max_interval_us: Option<i64>,
    #[serde(default)]
    pub parameter_header: Option<String>,
    #[serde(default)]
    pub internal_state_header: Option<String>,
    #[serde(default)]
    pub inputs: Vec<RunnableInputDescription>,
    #[serde(default)]
    pub outputs: Vec<RunnableOutputDescription>,
    pub build_information: BuildInformation,
}

/// Wrapper for a namespace string (e.g. "adas::perception")
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Namespace(pub String);

impl JsonSchema for Namespace {
    fn schema_name() -> String {
        "Namespace".to_string()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        gen.subschema_for::<String>()
    }
}

impl<'de> Deserialize<'de> for Namespace {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Ok(Namespace(String::new()));
        }
        // Validate namespace segments
        for segment in trimmed.split("::") {
            if segment.is_empty() {
                return Err(serde::de::Error::custom(
                    "Namespace must not have empty segments",
                ));
            }
        }
        Ok(Namespace(trimmed.to_string()))
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A collection of runnable archetypes within a namespace
#[derive(Debug, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunnableArchetypeList {
    pub namespace: Namespace,
    pub runnables: Vec<RunnableArchetypeDescription>,
}

impl RunnableArchetypeList {
    /// Parse a `RunnableArchetypeList` from a YAML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let list: RunnableArchetypeList = serde_yaml::from_reader(file)?;
        list.validate()?;
        Ok(list)
    }

    /// Parse a `RunnableArchetypeList` from a YAML string
    pub fn from_str(yaml: &str) -> Result<Self> {
        let list: RunnableArchetypeList = serde_yaml::from_str(yaml)?;
        list.validate()?;
        Ok(list)
    }

    /// Validate the parsed archetype list
    fn validate(&self) -> Result<()> {
        let mut unique_names = HashSet::new();
        for runnable in &self.runnables {
            if !unique_names.insert(&runnable.unique_name) {
                anyhow::bail!(
                    "Duplicate RunnableArchetypeDescription unique_name: {}",
                    runnable.unique_name
                );
            }
            if let (Some(min), Some(max)) = (runnable.min_interval_us, runnable.max_interval_us) {
                if min > max {
                    anyhow::bail!(
                        "min_interval_us ({}) > max_interval_us ({}) for '{}'",
                        min,
                        max,
                        runnable.unique_name
                    );
                }
            }
        }
        Ok(())
    }

    /// Find an archetype by unique_name (with or without namespace prefix)
    pub fn find_archetype(&self, name: &str) -> Option<&RunnableArchetypeDescription> {
        self.runnables.iter().find(|a| {
            a.unique_name == name
                || format!("{}::{}", self.namespace, a.unique_name) == name
        })
    }

    /// Find an output interface on a given archetype
    pub fn find_output_interface(
        &self,
        archetype_name: &str,
        interface_name: &str,
    ) -> Option<&RunnableOutputDescription> {
        self.find_archetype(archetype_name)
            .and_then(|a| a.outputs.iter().find(|o| o.unique_name == interface_name))
    }

    /// Find an input interface on a given archetype
    pub fn find_input_interface(
        &self,
        archetype_name: &str,
        interface_name: &str,
    ) -> Option<&RunnableInputDescription> {
        self.find_archetype(archetype_name)
            .and_then(|a| a.inputs.iter().find(|i| i.unique_name == interface_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_YAML: &str = r#"
namespace: ADAS

runnables:
  - unique_name: CameraController
    runnabletype: CPU
    wcet_us: 5000
    min_interval_us: 33333
    max_interval_us: 50000
    parameter_header: "parameters/CameraControllerParameters.yaml"
    internal_state_header: "internal_states/CameraState.yaml"
    outputs:
      - unique_name: camera_frames
        type_name: "adas::perception::CameraFrame"
        n_samples_max: 1
        n_slots: 3
        memory_backend: PosixShm
    build_information:
      bazel_target: "src/ADAS/CameraController:CameraController"

  - unique_name: BrakeController
    runnabletype: CPU
    wcet_us: 2000
    min_interval_us: 10000
    max_interval_us: 20000
    parameter_header: "parameters/BrakeControllerParameters.yaml"
    internal_state_header: "internal_states/BrakeControllerState.yaml"
    inputs:
      - unique_name: object_detections
        type_name: "adas::perception::ObjectDetections"
        n_samples_min: 1
        n_samples_max: 1
        policy: kNewestN
    outputs:
      - unique_name: brake_commands
        type_name: "adas::perception::BrakeCommand"
        n_samples_max: 1
        n_slots: 2
        memory_backend: PosixShm
    build_information:
      bazel_target: "src/ADAS/BrakeController:BrakeController"
"#;

    #[test]
    fn test_parse_from_str() {
        let list = RunnableArchetypeList::from_str(SAMPLE_YAML).unwrap();
        assert_eq!(list.namespace, Namespace("ADAS".to_string()));
        assert_eq!(list.runnables.len(), 2);
        assert_eq!(list.runnables[0].unique_name, "CameraController");
        assert_eq!(list.runnables[1].unique_name, "BrakeController");
    }

    #[test]
    fn test_find_archetype() {
        let list = RunnableArchetypeList::from_str(SAMPLE_YAML).unwrap();
        assert!(list.find_archetype("CameraController").is_some());
        assert!(list.find_archetype("ADAS::CameraController").is_some());
        assert!(list.find_archetype("NonExistent").is_none());
    }

    #[test]
    fn test_find_output_interface() {
        let list = RunnableArchetypeList::from_str(SAMPLE_YAML).unwrap();
        let output = list.find_output_interface("CameraController", "camera_frames");
        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.type_name, "adas::perception::CameraFrame");
        assert_eq!(output.n_samples_max, 1);
        assert_eq!(output.n_slots, Some(3));
    }

    #[test]
    fn test_find_input_interface() {
        let list = RunnableArchetypeList::from_str(SAMPLE_YAML).unwrap();
        let input = list.find_input_interface("BrakeController", "object_detections");
        assert!(input.is_some());
        let input = input.unwrap();
        assert_eq!(input.type_name, "adas::perception::ObjectDetections");
        assert_eq!(input.policy, BatchPolicy::NewestN);
    }

    #[test]
    fn test_reject_duplicate_names() {
        let yaml = r#"
namespace: Test
runnables:
  - unique_name: Dup
    runnabletype: CPU
    build_information:
      bazel_target: "a:b"
  - unique_name: Dup
    runnabletype: CPU
    build_information:
      bazel_target: "a:b"
"#;
        assert!(RunnableArchetypeList::from_str(yaml).is_err());
    }

    #[test]
    fn test_reject_min_greater_than_max_interval() {
        let yaml = r#"
namespace: Test
runnables:
  - unique_name: Bad
    runnabletype: CPU
    min_interval_us: 50000
    max_interval_us: 10000
    build_information:
      bazel_target: "a:b"
"#;
        assert!(RunnableArchetypeList::from_str(yaml).is_err());
    }

    #[test]
    fn test_parse_test_example_file() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../score-examples/test-example/graph_yaml/archetypes.yaml");
        if path.exists() {
            let list = RunnableArchetypeList::from_file(&path).unwrap();
            assert_eq!(list.namespace, Namespace("adas".to_string()));
            assert_eq!(list.runnables.len(), 6);
            // Verify all expected archetypes are present
            assert!(list.find_archetype("CameraController").is_some());
            assert!(list.find_archetype("NeuralNetInference").is_some());
            assert!(list.find_archetype("TrafficSignRecognition").is_some());
            assert!(list.find_archetype("BrakeController").is_some());
            assert!(list.find_archetype("SystemLogger").is_some());
            assert!(list.find_archetype("PerformanceTracer").is_some());
        }
    }
}

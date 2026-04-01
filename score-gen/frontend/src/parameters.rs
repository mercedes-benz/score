use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;

use crate::interfaces::StructMemberDefinition;

/// A parameter definition file for a runnable archetype.
///
/// Each parameter file defines a single struct with typed members and default
/// values, representing the configuration parameters for a specific runnable.
#[derive(Deserialize, Clone, PartialEq, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParameterDefinition {
    pub name: String,
    #[serde(default)]
    pub top_level: bool,
    pub members: Vec<StructMemberDefinition>,
}

impl ParameterDefinition {
    /// Parse a `ParameterDefinition` from a YAML file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let def = Self::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(def)
    }

    /// Parse a `ParameterDefinition` from a YAML string.
    pub fn from_str(yaml: &str) -> Result<Self> {
        let def: ParameterDefinition = serde_yaml::from_str(yaml)?;
        def.validate()?;
        Ok(def)
    }

    fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            anyhow::bail!("Parameter name must not be empty");
        }
        let mut member_names = HashSet::new();
        for m in &self.members {
            if !member_names.insert(&m.name) {
                anyhow::bail!(
                    "Duplicate member '{}' in parameter '{}'",
                    m.name,
                    self.name
                );
            }
        }
        Ok(())
    }

    /// Find a member by name.
    pub fn find_member(&self, name: &str) -> Option<&StructMemberDefinition> {
        self.members.iter().find(|m| m.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_parameters() {
        let yaml = r#"
name: MyParams
members:
  - name: threshold
    type: float
    default: 0.5
"#;
        let def = ParameterDefinition::from_str(yaml).unwrap();
        assert_eq!(def.name, "MyParams");
        assert_eq!(def.members.len(), 1);
        assert_eq!(def.members[0].name, "threshold");
        assert_eq!(def.members[0].member_type, "float");
    }

    #[test]
    fn test_parse_with_multiple_members() {
        let yaml = r#"
name: CameraParams
top_level: true
members:
  - name: frame_rate
    type: uint32_t
    default: 30
  - name: exposure_auto
    type: bool
    default: true
  - name: model_path
    type: string
    capacity: 256
    default: "/models/default.onnx"
"#;
        let def = ParameterDefinition::from_str(yaml).unwrap();
        assert_eq!(def.name, "CameraParams");
        assert!(def.top_level);
        assert_eq!(def.members.len(), 3);
        assert_eq!(def.members[2].capacity, Some(256));
    }

    #[test]
    fn test_parse_with_constraints() {
        let yaml = r#"
name: BrakeParams
members:
  - name: max_pressure
    type: float
    default: 200.0
    constraints:
      min: 0.0
      max: 500.0
"#;
        let def = ParameterDefinition::from_str(yaml).unwrap();
        let member = &def.members[0];
        let constraints = member.constraints.as_ref().unwrap();
        assert_eq!(constraints.min, Some(0.0));
        assert_eq!(constraints.max, Some(500.0));
    }

    #[test]
    fn test_find_member() {
        let yaml = r#"
name: Params
members:
  - name: alpha
    type: float
    default: 1.0
  - name: beta
    type: uint32_t
    default: 10
"#;
        let def = ParameterDefinition::from_str(yaml).unwrap();
        assert!(def.find_member("alpha").is_some());
        assert!(def.find_member("beta").is_some());
        assert!(def.find_member("gamma").is_none());
    }

    #[test]
    fn test_reject_empty_name() {
        let yaml = r#"
name: ""
members:
  - name: x
    type: float
"#;
        assert!(ParameterDefinition::from_str(yaml).is_err());
    }

    #[test]
    fn test_reject_duplicate_members() {
        let yaml = r#"
name: Bad
members:
  - name: x
    type: float
  - name: x
    type: uint32_t
"#;
        assert!(ParameterDefinition::from_str(yaml).is_err());
    }

    #[test]
    fn test_parse_test_example_file() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../score-examples/test-example/graph_yaml/parameters/CameraControllerParameters.yaml");
        if path.exists() {
            let def = ParameterDefinition::from_file(&path).unwrap();
            assert_eq!(def.name, "CameraControllerParameters");
            assert!(def.top_level);
            assert!(!def.members.is_empty());
            assert!(def.find_member("frame_rate_fps").is_some());
            assert!(def.find_member("resolution_width").is_some());
        }
    }

    #[test]
    fn test_parse_all_test_example_files() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../score-examples/test-example/graph_yaml/parameters");
        if dir.exists() {
            let mut count = 0;
            for entry in std::fs::read_dir(&dir).unwrap() {
                let entry = entry.unwrap();
                if entry.path().extension().map_or(false, |e| e == "yaml") {
                    let def = ParameterDefinition::from_file(&entry.path()).unwrap();
                    assert!(!def.name.is_empty());
                    assert!(!def.members.is_empty());
                    count += 1;
                }
            }
            assert_eq!(count, 6);
        }
    }
}

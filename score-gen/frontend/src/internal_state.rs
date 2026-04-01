use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;

use crate::interfaces::StructMemberDefinition;

/// An internal state definition for a runnable archetype.
///
/// Each internal state file defines a single struct with typed members and
/// optional default values, representing the private runtime state of a
/// specific runnable.
#[derive(Deserialize, Clone, PartialEq, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InternalStateDefinition {
    pub name: String,
    #[serde(default)]
    pub top_level: bool,
    pub members: Vec<StructMemberDefinition>,
}

impl InternalStateDefinition {
    /// Parse an `InternalStateDefinition` from a YAML file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let def = Self::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(def)
    }

    /// Parse an `InternalStateDefinition` from a YAML string.
    pub fn from_str(yaml: &str) -> Result<Self> {
        let def: InternalStateDefinition = serde_yaml::from_str(yaml)?;
        def.validate()?;
        Ok(def)
    }

    fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            anyhow::bail!("Internal state name must not be empty");
        }
        let mut member_names = HashSet::new();
        for m in &self.members {
            if !member_names.insert(&m.name) {
                anyhow::bail!(
                    "Duplicate member '{}' in internal state '{}'",
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
    fn test_parse_minimal_state() {
        let yaml = r#"
name: MyState
members:
  - name: counter
    type: uint64_t
    default: 0
"#;
        let def = InternalStateDefinition::from_str(yaml).unwrap();
        assert_eq!(def.name, "MyState");
        assert_eq!(def.members.len(), 1);
        assert_eq!(def.members[0].name, "counter");
    }

    #[test]
    fn test_parse_with_time_point() {
        let yaml = r#"
name: TimedState
top_level: true
members:
  - name: last_timestamp
    type: time_point
  - name: is_active
    type: bool
    default: false
"#;
        let def = InternalStateDefinition::from_str(yaml).unwrap();
        assert!(def.top_level);
        assert_eq!(def.members.len(), 2);
        assert_eq!(def.members[0].member_type, "time_point");
        assert!(def.members[0].default.is_none());
    }

    #[test]
    fn test_parse_with_array_member() {
        let yaml = r#"
name: BufferedState
members:
  - name: buffer
    type: float[1024]
  - name: index
    type: uint32_t
    default: 0
"#;
        let def = InternalStateDefinition::from_str(yaml).unwrap();
        assert_eq!(def.members[0].member_type, "float[1024]");
    }

    #[test]
    fn test_find_member() {
        let yaml = r#"
name: State
members:
  - name: alpha
    type: float
  - name: beta
    type: uint32_t
    default: 0
"#;
        let def = InternalStateDefinition::from_str(yaml).unwrap();
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
        assert!(InternalStateDefinition::from_str(yaml).is_err());
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
        assert!(InternalStateDefinition::from_str(yaml).is_err());
    }

    #[test]
    fn test_parse_test_example_file() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../score-examples/test-example/graph_yaml/internal_states/CameraState.yaml");
        if path.exists() {
            let def = InternalStateDefinition::from_file(&path).unwrap();
            assert_eq!(def.name, "CameraState");
            assert!(def.top_level);
            assert!(!def.members.is_empty());
            assert!(def.find_member("frame_counter").is_some());
            assert!(def.find_member("is_streaming").is_some());
        }
    }

    #[test]
    fn test_parse_all_test_example_files() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../score-examples/test-example/graph_yaml/internal_states");
        if dir.exists() {
            let mut count = 0;
            for entry in std::fs::read_dir(&dir).unwrap() {
                let entry = entry.unwrap();
                if entry.path().extension().map_or(false, |e| e == "yaml") {
                    let def = InternalStateDefinition::from_file(&entry.path()).unwrap();
                    assert!(!def.name.is_empty());
                    assert!(!def.members.is_empty());
                    count += 1;
                }
            }
            assert_eq!(count, 6);
        }
    }
}

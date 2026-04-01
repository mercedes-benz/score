use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;

/// A constraint on a numeric struct member (min/max bounds).
#[derive(Deserialize, Clone, PartialEq, Debug, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MemberConstraints {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// A single member (field) within a struct definition.
#[derive(Deserialize, Clone, PartialEq, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StructMemberDefinition {
    pub name: String,
    #[serde(alias = "type")]
    pub member_type: String,
    #[serde(default)]
    #[schemars(with = "Option<serde_json::Value>")]
    pub default: Option<serde_yaml::Value>,
    #[serde(default)]
    pub capacity: Option<u64>,
    #[serde(default)]
    pub constraints: Option<MemberConstraints>,
}

/// A single enum value entry.
#[derive(Deserialize, Clone, PartialEq, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnumEntry {
    pub name: String,
    pub value: Option<i64>,
}

/// An enum definition with a base type and named values.
#[derive(Deserialize, Clone, PartialEq, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnumDefinition {
    pub name: String,
    pub base: String,
    pub values: Vec<EnumEntry>,
}

/// A constexpr value definition.
#[derive(Deserialize, Clone, PartialEq, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstexprDefinition {
    pub name: String,
    pub member_type: String,
    #[schemars(with = "serde_json::Value")]
    pub value: serde_yaml::Value,
}

/// A struct (type) definition within the interface list.
#[derive(Deserialize, Clone, PartialEq, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InterfaceStructDefinition {
    pub name: String,
    #[serde(default)]
    pub top_level: bool,
    pub members: Vec<StructMemberDefinition>,
}

/// Top-level interface list parsed from an `interfaces.yaml` file.
///
/// Contains struct definitions, enum definitions, and constexpr values,
/// all scoped under a C++ namespace.
#[derive(Deserialize, Clone, PartialEq, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InterfaceList {
    pub namespace: String,
    #[serde(default)]
    pub structs: Vec<InterfaceStructDefinition>,
    #[serde(default)]
    pub enums: Vec<EnumDefinition>,
    #[serde(default)]
    pub constexprs: Vec<ConstexprDefinition>,
}

impl InterfaceList {
    /// Parse an `InterfaceList` from a YAML file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let list = Self::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(list)
    }

    /// Parse an `InterfaceList` from a YAML string.
    pub fn from_str(yaml: &str) -> Result<Self> {
        let list: InterfaceList = serde_yaml::from_str(yaml)?;
        list.validate()?;
        Ok(list)
    }

    fn validate(&self) -> Result<()> {
        if self.namespace.is_empty() {
            anyhow::bail!("namespace must not be empty");
        }

        // Check for duplicate struct names
        let mut struct_names = HashSet::new();
        for s in &self.structs {
            if !struct_names.insert(&s.name) {
                anyhow::bail!("Duplicate struct name: {}", s.name);
            }
            self.validate_struct(s)?;
        }

        // Check for duplicate enum names
        let mut enum_names = HashSet::new();
        for e in &self.enums {
            if !enum_names.insert(&e.name) {
                anyhow::bail!("Duplicate enum name: {}", e.name);
            }
            self.validate_enum(e)?;
        }

        // Check for duplicate constexpr names
        let mut constexpr_names = HashSet::new();
        for c in &self.constexprs {
            if !constexpr_names.insert(&c.name) {
                anyhow::bail!("Duplicate constexpr name: {}", c.name);
            }
        }

        Ok(())
    }

    fn validate_struct(&self, s: &InterfaceStructDefinition) -> Result<()> {
        if s.name.is_empty() {
            anyhow::bail!("Struct name must not be empty");
        }
        let mut member_names = HashSet::new();
        for m in &s.members {
            if !member_names.insert(&m.name) {
                anyhow::bail!("Duplicate member '{}' in struct '{}'", m.name, s.name);
            }
        }
        Ok(())
    }

    fn validate_enum(&self, e: &EnumDefinition) -> Result<()> {
        if e.name.is_empty() {
            anyhow::bail!("Enum name must not be empty");
        }
        let mut value_names = HashSet::new();
        for v in &e.values {
            if !value_names.insert(&v.name) {
                anyhow::bail!("Duplicate value '{}' in enum '{}'", v.name, e.name);
            }
        }
        Ok(())
    }

    /// Find a struct definition by name.
    pub fn find_struct(&self, name: &str) -> Option<&InterfaceStructDefinition> {
        self.structs.iter().find(|s| s.name == name)
    }

    /// Find an enum definition by name.
    pub fn find_enum(&self, name: &str) -> Option<&EnumDefinition> {
        self.enums.iter().find(|e| e.name == name)
    }

    /// Return only top-level struct definitions (message types).
    pub fn top_level_structs(&self) -> Vec<&InterfaceStructDefinition> {
        self.structs.iter().filter(|s| s.top_level).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_interface() {
        let yaml = r#"
namespace: my::ns

structs:
  - name: MyStruct
    top_level: true
    members:
      - name: value
        type: uint32_t
        default: 0
"#;
        let list = InterfaceList::from_str(yaml).unwrap();
        assert_eq!(list.namespace, "my::ns");
        assert_eq!(list.structs.len(), 1);
        assert_eq!(list.structs[0].name, "MyStruct");
        assert!(list.structs[0].top_level);
        assert_eq!(list.structs[0].members[0].name, "value");
        assert_eq!(list.structs[0].members[0].member_type, "uint32_t");
    }

    #[test]
    fn test_parse_enums() {
        let yaml = r#"
namespace: test

enums:
  - name: Color
    base: uint8_t
    values:
      - name: kRed
        value: 0
      - name: kGreen
        value: 1
      - name: kBlue
        value: 2
"#;
        let list = InterfaceList::from_str(yaml).unwrap();
        assert_eq!(list.enums.len(), 1);
        assert_eq!(list.enums[0].name, "Color");
        assert_eq!(list.enums[0].base, "uint8_t");
        assert_eq!(list.enums[0].values.len(), 3);
        assert_eq!(list.enums[0].values[1].name, "kGreen");
        assert_eq!(list.enums[0].values[1].value, Some(1));
    }

    #[test]
    fn test_parse_constexprs() {
        let yaml = r#"
namespace: test

constexprs:
  - name: MaxSpeed
    member_type: float
    value: 250.0
  - name: FrameRate
    member_type: uint32_t
    value: 30
"#;
        let list = InterfaceList::from_str(yaml).unwrap();
        assert_eq!(list.constexprs.len(), 2);
        assert_eq!(list.constexprs[0].name, "MaxSpeed");
        assert_eq!(list.constexprs[1].name, "FrameRate");
    }

    #[test]
    fn test_parse_member_with_constraints() {
        let yaml = r#"
namespace: test

structs:
  - name: Sensor
    top_level: true
    members:
      - name: pressure
        type: float
        default: 0.0
        constraints:
          min: 0.0
          max: 200.0
"#;
        let list = InterfaceList::from_str(yaml).unwrap();
        let member = &list.structs[0].members[0];
        let constraints = member.constraints.as_ref().unwrap();
        assert_eq!(constraints.min, Some(0.0));
        assert_eq!(constraints.max, Some(200.0));
    }

    #[test]
    fn test_parse_array_type() {
        let yaml = r#"
namespace: test

structs:
  - name: Buffer
    top_level: true
    members:
      - name: data
        type: uint8_t[1024]
"#;
        let list = InterfaceList::from_str(yaml).unwrap();
        assert_eq!(list.structs[0].members[0].member_type, "uint8_t[1024]");
    }

    #[test]
    fn test_parse_string_with_capacity() {
        let yaml = r#"
namespace: test

structs:
  - name: Message
    top_level: true
    members:
      - name: text
        type: string
        capacity: 256
"#;
        let list = InterfaceList::from_str(yaml).unwrap();
        assert_eq!(list.structs[0].members[0].capacity, Some(256));
    }

    #[test]
    fn test_find_struct() {
        let yaml = r#"
namespace: test

structs:
  - name: Alpha
    top_level: true
    members:
      - name: x
        type: uint32_t
  - name: Beta
    top_level: false
    members:
      - name: y
        type: float
"#;
        let list = InterfaceList::from_str(yaml).unwrap();
        assert!(list.find_struct("Alpha").is_some());
        assert!(list.find_struct("Beta").is_some());
        assert!(list.find_struct("Gamma").is_none());
    }

    #[test]
    fn test_top_level_structs() {
        let yaml = r#"
namespace: test

structs:
  - name: TopLevel
    top_level: true
    members:
      - name: x
        type: uint32_t
  - name: SubComponent
    top_level: false
    members:
      - name: y
        type: float
  - name: AnotherTopLevel
    top_level: true
    members:
      - name: z
        type: uint8_t
"#;
        let list = InterfaceList::from_str(yaml).unwrap();
        let top = list.top_level_structs();
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].name, "TopLevel");
        assert_eq!(top[1].name, "AnotherTopLevel");
    }

    #[test]
    fn test_reject_duplicate_struct_names() {
        let yaml = r#"
namespace: test

structs:
  - name: Foo
    members:
      - name: x
        type: uint32_t
  - name: Foo
    members:
      - name: y
        type: float
"#;
        assert!(InterfaceList::from_str(yaml).is_err());
    }

    #[test]
    fn test_reject_duplicate_enum_names() {
        let yaml = r#"
namespace: test

enums:
  - name: Color
    base: uint8_t
    values:
      - name: kRed
        value: 0
  - name: Color
    base: uint8_t
    values:
      - name: kBlue
        value: 1
"#;
        assert!(InterfaceList::from_str(yaml).is_err());
    }

    #[test]
    fn test_reject_duplicate_member_names() {
        let yaml = r#"
namespace: test

structs:
  - name: Bad
    members:
      - name: x
        type: uint32_t
      - name: x
        type: float
"#;
        assert!(InterfaceList::from_str(yaml).is_err());
    }

    #[test]
    fn test_parse_test_example_file() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../score-examples/test-example/graph_yaml/interfaces.yaml");
        if path.exists() {
            let list = InterfaceList::from_file(&path).unwrap();
            assert_eq!(list.namespace, "adas::perception");
            assert!(!list.structs.is_empty());
            assert!(!list.enums.is_empty());
            assert!(!list.constexprs.is_empty());
            // Verify key types exist
            assert!(list.find_struct("CameraFrame").is_some());
            assert!(list.find_struct("ObjectDetections").is_some());
            assert!(list.find_struct("BrakeCommand").is_some());
            assert!(list.find_enum("ObjectClass").is_some());
            // Verify top-level filtering
            let top = list.top_level_structs();
            assert!(top.iter().any(|s| s.name == "CameraFrame"));
            assert!(top.iter().all(|s| s.name != "Time")); // Time is not top-level
        }
    }
}

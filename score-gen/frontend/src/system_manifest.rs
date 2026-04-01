use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// The system manifest that ties together all YAML graph definition files.
#[derive(Debug, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct SystemManifest {
    pub system_name: String,
    #[serde(default)]
    pub interfaces: Vec<String>,
    #[serde(default)]
    pub archetypes: Vec<String>,
    #[serde(default)]
    pub instances: Vec<String>,
}

impl SystemManifest {
    /// Parse a `SystemManifest` from a YAML file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("Failed to open {}", path.display()))?;
        let manifest: SystemManifest = serde_yaml::from_reader(file)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Parse a `SystemManifest` from a YAML string.
    pub fn from_str(yaml: &str) -> Result<Self> {
        let manifest: SystemManifest = serde_yaml::from_str(yaml)?;
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> Result<()> {
        if self.system_name.is_empty() {
            anyhow::bail!("system_name must not be empty");
        }
        Ok(())
    }

    /// Resolve archetype file paths relative to the manifest's parent directory.
    pub fn archetype_paths(&self, base_dir: &Path) -> Vec<PathBuf> {
        self.archetypes.iter().map(|p| base_dir.join(p)).collect()
    }

    /// Resolve interface file paths relative to the manifest's parent directory.
    pub fn interface_paths(&self, base_dir: &Path) -> Vec<PathBuf> {
        self.interfaces.iter().map(|p| base_dir.join(p)).collect()
    }

    /// Resolve instance file paths relative to the manifest's parent directory.
    pub fn instance_paths(&self, base_dir: &Path) -> Vec<PathBuf> {
        self.instances.iter().map(|p| base_dir.join(p)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_manifest() {
        let yaml = r#"
system_name: "test_example"

interfaces:
  - "interfaces.yaml"

archetypes:
  - "archetypes.yaml"

instances:
  - "instances.yaml"
"#;
        let manifest = SystemManifest::from_str(yaml).unwrap();
        assert_eq!(manifest.system_name, "test_example");
        assert_eq!(manifest.interfaces, vec!["interfaces.yaml"]);
        assert_eq!(manifest.archetypes, vec!["archetypes.yaml"]);
        assert_eq!(manifest.instances, vec!["instances.yaml"]);
    }

    #[test]
    fn test_parse_minimal_manifest() {
        let yaml = r#"
system_name: "test-example"

archetypes:
  - "archetypes.yaml"
"#;
        let manifest = SystemManifest::from_str(yaml).unwrap();
        assert_eq!(manifest.system_name, "test-example");
        assert_eq!(manifest.archetypes, vec!["archetypes.yaml"]);
        assert!(manifest.interfaces.is_empty());
        assert!(manifest.instances.is_empty());
    }

    #[test]
    fn test_reject_empty_system_name() {
        let yaml = r#"
system_name: ""
archetypes:
  - "archetypes.yaml"
"#;
        assert!(SystemManifest::from_str(yaml).is_err());
    }

    #[test]
    fn test_resolve_paths() {
        let yaml = r#"
system_name: "test"
archetypes:
  - "archetypes.yaml"
  - "more/extra_archetypes.yaml"
interfaces:
  - "interfaces.yaml"
"#;
        let manifest = SystemManifest::from_str(yaml).unwrap();
        let base = Path::new("/project/graph_yaml");

        let arch_paths = manifest.archetype_paths(base);
        assert_eq!(arch_paths[0], PathBuf::from("/project/graph_yaml/archetypes.yaml"));
        assert_eq!(arch_paths[1], PathBuf::from("/project/graph_yaml/more/extra_archetypes.yaml"));

        let iface_paths = manifest.interface_paths(base);
        assert_eq!(iface_paths[0], PathBuf::from("/project/graph_yaml/interfaces.yaml"));
    }

    #[test]
    fn test_parse_test_example_file() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../score-examples/test-example/graph_yaml/system_manifest.yaml");
        if path.exists() {
            let manifest = SystemManifest::from_file(&path).unwrap();
            assert_eq!(manifest.system_name, "test-example");
            assert_eq!(manifest.archetypes, vec!["archetypes.yaml"]);
        }
    }
}

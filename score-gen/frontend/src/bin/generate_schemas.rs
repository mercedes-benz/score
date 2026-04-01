//! Generates JSON Schema files from the frontend parser Rust types.
//!
//! Usage: generate_schemas [output_dir]
//!   output_dir defaults to frontend/schemas/

use schemars::schema_for;
use std::fs;
use std::path::PathBuf;

fn main() {
    let output_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("frontend/schemas"));

    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    let schemas: Vec<(&str, schemars::schema::RootSchema)> = vec![
        (
            "system_manifest_schema.json",
            schema_for!(frontend::system_manifest::SystemManifest),
        ),
        (
            "runnable_archetype_list_schema.json",
            schema_for!(frontend::archetypes::RunnableArchetypeList),
        ),
        (
            "interface_list_schema.json",
            schema_for!(frontend::interfaces::InterfaceList),
        ),
        (
            "runnable_instance_list_schema.json",
            schema_for!(frontend::instances::InstanceList),
        ),
        (
            "parameter_definition_schema.json",
            schema_for!(frontend::parameters::ParameterDefinition),
        ),
        (
            "internal_state_definition_schema.json",
            schema_for!(frontend::internal_state::InternalStateDefinition),
        ),
    ];

    for (filename, schema) in &schemas {
        let json = serde_json::to_string_pretty(schema).expect("Failed to serialize schema");
        let path = output_dir.join(filename);
        fs::write(&path, format!("{}\n", json)).expect("Failed to write schema file");
        println!("Generated {}", path.display());
    }
}

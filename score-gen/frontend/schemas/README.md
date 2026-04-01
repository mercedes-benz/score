# JSON Schemas

These JSON Schema files are **auto-generated** from the Rust parser structs using
[schemars](https://docs.rs/schemars). This ensures they stay in sync with the actual
parser code. They are provided for **IDE autocompletion and inline validation** when
editing YAML files.

## Generating schemas

To regenerate after modifying the frontend parser structs:

```bash
# From score-gen root (with cargo)
cargo run --package frontend --bin generate_schemas -- frontend/schemas/

# From score-gen root (with bazel)
bazel run //frontend:generate_schemas -- $PWD/frontend/schemas/
```

## Available schemas

| Schema | Validates |
|--------|-----------|
| `system_manifest_schema.json` | `system_manifest.yaml` — the root manifest listing all graph files |
| `runnable_archetype_list_schema.json` | `archetypes.yaml` — runnable archetype definitions |
| `interface_list_schema.json` | `interfaces.yaml` — struct, enum, and constexpr definitions |
| `runnable_instance_list_schema.json` | `instances.yaml` — runnable instance definitions with parameter overrides |
| `parameter_definition_schema.json` | `parameters/*.yaml` — parameter struct definitions for runnables |
| `internal_state_definition_schema.json` | `internal_states/*.yaml` — internal state struct definitions for runnables |

## IDE setup

### VS Code

Install the [YAML extension](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml)
by Red Hat, then add the following to your `.vscode/settings.json`:

```json
{
  "yaml.schemas": {
    "frontend/schemas/system_manifest_schema.json": "system_manifest.yaml",
    "frontend/schemas/runnable_archetype_list_schema.json": "archetypes.yaml",
    "frontend/schemas/interface_list_schema.json": "interfaces.yaml",
    "frontend/schemas/runnable_instance_list_schema.json": "instances.yaml",
    "frontend/schemas/parameter_definition_schema.json": "parameters/*.yaml",
    "frontend/schemas/internal_state_definition_schema.json": "internal_states/*.yaml"
  }
}
```

### Per-file association

You can also associate a schema directly in a YAML file by adding a modeline comment
at the top:

```yaml
# yaml-language-server: $schema=../../score-gen/frontend/schemas/system_manifest_schema.json
system_name: "my-project"
...
```

This gives you autocompletion, hover documentation, and validation errors directly
in the editor.

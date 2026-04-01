# score-gen

A Rust-based code generation tool that parses YAML graph definitions and
generates C++ skeleton code, Bazel BUILD files, and other artifacts for a
middleware framework.

Supports parsing **system manifests**, **interfaces**, **runnable archetypes**,
**runnable instances**, **parameters**, and **internal state** definitions.

For the full YAML modeling language specification, see [frontend/docs/yaml_reference.md](frontend/docs/yaml_reference.md).

JSON schemas for IDE autocompletion and validation are auto-generated from parser structs via [schemars](https://docs.rs/schemars) — see [frontend/schemas/](frontend/schemas/README.md).

## Building

### With Bazel (recommended)

```bash
cd score-gen
bazel build //:scoregen
```

## Usage

```bash
scoregen [OPTIONS] <manifest.yaml>
```

Where `<manifest.yaml>` is the path to a system manifest YAML file. All files
referenced in the manifest (interfaces, archetypes, instances, parameters,
internal states) are resolved relative to the manifest's directory.

### Options

- `-h, --help` — Print help message
- `--graphviz=<path>` — Generate a Graphviz DOT file of the dataflow graph

### Example

```bash
cd score-gen
bazel run //:scoregen -- $PWD/graph_yaml/system_manifest.yaml
bazel run //:scoregen -- --graphviz=$PWD/graph.gv $PWD/graph_yaml/system_manifest.yaml
```

## System manifest format

The `system_manifest.yaml` file lists which YAML files to parse:

```yaml
system_name: "my-project"

interfaces:
  - "interfaces.yaml"

archetypes:
  - "archetypes.yaml"

instances:
  - "instances.yaml"
```

Paths are relative to the directory containing the manifest. Parameters and
internal state files are referenced from within each archetype definition
(via `parameter_header` and `internal_state_header` fields) rather than listed
in the manifest directly.

## What gets generated

| Output | Location | Content |
|--------|----------|---------|
| Runnable skeletons | `src-gen/<namespace>/<runnable>/` | `.hpp.skeleton`, `.cpp.skeleton`, `BUILD.bazel.skeleton`, `unit_test.cpp.skeleton` |
| Interface headers | `src-gen/interfaces/` | One `.hpp` per struct, enum, and constexpr |
| Task binaries | `src-gen/<instance_ns>/` | Per-instance `main()` wrappers and `BUILD.bazel` |
| Parameter headers | `src-gen/parameter/` | Per-runnable parameter struct (global scope) |
| Internal state headers | `src-gen/internal_state/` | Per-runnable internal state struct (global scope) |
| MODULE.bazel | project root | Bazel module with `score-runtime` dependency |

### Generated C++ API

Each runnable archetype generates `on_init()` and `on_update()` functions. The
`on_update()` signature includes:

- **`const Parameters&`** — immutable parameters (when defined)
- **`const Inputs&`** — input deques of `const T*` pointers (when inputs exist)
- **`Outputs&`** — output deques of `std::pair<T*, std::size_t>` (when outputs exist)
- **`InternalState&`** — mutable internal state (when defined)

Const references appear first in the argument list, followed by non-const references.

### Input/Output queue details

- **Inputs**: `std::deque<const T*>` — read-only pointers, default-initialized
  to `nullptr` with capacity equal to `n_samples_max` (defaults to 1).
- **Outputs**: `std::deque<std::pair<T*, std::size_t>>` — read-write pointers
  paired with `n_slots`. Default-initialized with capacity equal to
  `n_samples_max`.
- **Default `n_slots`**: When not specified, computed as
  `max(consumer_n_samples_max) × 2 + producer_n_samples_max`.
- **Dangling type validation**: All input/output `type_name` values are
  validated against declared interface structs at generation time.

## Project structure

```
score-gen/
├── frontend/          # YAML parsers
│   ├── src/
│   │   ├── archetypes.rs       # Runnable archetype parser
│   │   ├── interfaces.rs       # Interface (structs, enums, constexprs) parser
│   │   ├── instances.rs        # Runnable instance parser
│   │   ├── parameters.rs       # Parameter definition parser
│   │   ├── internal_state.rs   # Internal state definition parser
│   │   ├── system_manifest.rs  # System manifest parser
│   │   └── lib.rs
│   ├── docs/
│   │   └── yaml_reference.md   # Full YAML modeling language reference
│   └── schemas/                # Auto-generated JSON schemas for IDE support
├── backend/           # C++ code generation
│   ├── src/codegen/
│   │   ├── mod.rs              # Shared utilities (snake_case, type mapping, render)
│   │   ├── generate.rs         # Runnable skeleton generation
│   │   ├── generate_tasks.rs   # Task binary generation
│   │   ├── interfaces.rs       # Interface header generation
│   │   ├── module_bazel.rs     # MODULE.bazel generation
│   │   ├── runnable.rs         # RunnableContext (inputs, outputs, parameters)
│   │   ├── task.rs             # TaskContext
│   │   ├── parameters.rs       # ParameterContext
│   │   └── internal_state.rs   # InternalStateContext
│   └── templates/              # MiniJinja templates for C++ code
├── util/              # Utilities
│   └── src/
│       ├── graphviz.rs         # Graphviz DOT generation
│       └── lib.rs
└── src/
    └── main.rs        # CLI entry point
```


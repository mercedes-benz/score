# score-examples

Example projects demonstrating how to use [score-gen](../score-gen/) to generate C++ skeleton code from YAML graph definitions.

## Prerequisites

- Bazel 9+ with Bzlmod enabled
- [score-gen](../score-gen/) checked out as a sibling directory
- [score-runtime](../score-runtime/) checked out as a sibling directory

## Quick start

```bash
cd test-example

# 1. Generate code from YAML definitions
bazel run @score-gen//:scoregen -- $PWD/graph_yaml/system_manifest.yaml

# 2. Activate skeleton files (remove .skeleton extension)
find src-gen/ -name '*.skeleton' | while read F; do mv "${F}" "${F%.skeleton}"; done

# 3. Build everything
bazel build //src-gen/...

# 4. Run a task
bazel run //src-gen/mini_adas:camera_controller
```

## Examples

| Example | Description |
|---|---|
| [test-example](test-example/) | ADAS demo with 6 runnables, interfaces, parameter overrides, and task binaries |

## What score-gen produces

Given a `system_manifest.yaml`, score-gen generates:

- **Runnable libraries** — C++ `cc_library` per runnable with `on_init`/`on_update` skeletons, parameter structs, and internal state structs
- **Interface headers** — C++ structs, enums, and constexprs from shared interface definitions
- **Task binaries** — C++ `cc_binary` per instance with `main()` that wires parameters (with overrides) and calls the runnable
- **Bazel build files** — `BUILD.bazel` per runnable and per instance namespace, plus a `MODULE.bazel` for the project

## Adding a new example

1. Create a new directory under `score-examples/`:

   ```
   score-examples/
   └── my-example/
       ├── BUILD.bazel
       └── graph_yaml/
           ├── system_manifest.yaml
           ├── archetypes.yaml
           ├── interfaces.yaml
           ├── instances.yaml
           ├── parameters/
           └── internal_states/
   ```

2. Define your system in `system_manifest.yaml` referencing archetypes, interfaces, and instances YAML files.

3. Run:

   ```bash
   cd my-example
   bazel run @score-gen//:scoregen -- $PWD/graph_yaml/system_manifest.yaml
   find src-gen/ -name '*.skeleton' | while read F; do mv "${F}" "${F%.skeleton}"; done
   bazel build //src-gen/...
   ```
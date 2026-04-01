# test-example

An ADAS (Advanced Driver Assistance System) demo consisting of 6 runnables:

| Runnable | Type | Description |
|---|---|---|
| CameraController | CPU | Camera sensor source node (30 FPS) |
| NeuralNetInference | GPU | Object detection via neural network |
| TrafficSignRecognition | GPU | Traffic sign classification |
| BrakeController | CPU | Safety-critical brake actuator |
| SystemLogger | CPU | Centralized logging sink |
| PerformanceTracer | CPU | Performance monitoring sink |

## Prerequisites

- Bazel 9+ with Bzlmod enabled
- [score-gen](../../score-gen/) built via Bazel (sibling of `score-examples`)
- [score-runtime](../../score-runtime/) checked out as a sibling directory

## Step 1 — Generate the code

From the `score-gen` directory, run the code generator pointing to the system manifest:

```bash
cd ../../score-gen
bazel run //:scoregen -- /absolute/path/to/test-example/graph_yaml/system_manifest.yaml
```

This parses archetypes, interfaces, parameters, internal states, and instances from `graph_yaml/` and generates:

| Directory | Content |
|---|---|
| `src-gen/adas/<runnable>/` | Per-runnable C++ skeleton files (`.hpp.skeleton`, `.cpp.skeleton`, `BUILD.bazel.skeleton`, `unit_test.cpp.skeleton`) |
| `src-gen/interfaces/` | C++ interface headers (structs, enums, constexprs) |
| `src-gen/mini_adas/src/` | Per-instance task binaries (`<runnable>.cpp`) |
| `src-gen/mini_adas/BUILD.bazel` | Bazel `cc_binary` targets for all tasks |
| `src-gen/parameter/` | Parameter struct headers (global scope) |
| `src-gen/internal_state/` | Internal state struct headers (global scope) |
| `MODULE.bazel` | Bazel module definition with `score-runtime` dependency |

## Step 2 — Copy runnable skeletons to `src/`

All runnable archetype files are generated with a `.skeleton` extension to prevent overwriting user code on re-generation. To activate them, create a `src/` directory, copy the archetype namespace directory, and remove the `.skeleton` extensions:

```bash
mkdir -p src
cp -r src-gen/adas src/
find src/ -name '*.skeleton' -exec sh -c 'mv "$1" "${1%.skeleton}"' _ {} \;
```

> **Important:** Do **not** rename the `.skeleton` files in place inside `src-gen/`.
> The `src-gen/` directory is fully regenerated each time `scoregen` runs —
> any modifications made there will be lost. Always copy the skeletons to
> `src/` first, then remove the `.skeleton` extension in `src/`.

After this step, user-editable code lives in `src/` while fully-generated infrastructure stays in `src-gen/`.

## Step 3 — Build all libraries and tasks

```bash
bazel build //...
```

This compiles:
- 6 `cc_library` targets (one per runnable under `src/adas/`)
- 1 `cc_library` for shared parameter headers (`src-gen/parameter/`)
- 1 `cc_library` for shared internal state headers (`src-gen/internal_state/`)
- 6 `cc_binary` targets (one per task under `src-gen/mini_adas/`)
- 6 `cc_test` targets (one unit test per runnable under `src/adas/`)

To build a single runnable library:

```bash
bazel build //src/adas/camera_controller
```

To build a single task binary:

```bash
bazel build //src-gen/mini_adas:camera_controller
```

## Step 4 — Run a task

```bash
bazel run //src-gen/mini_adas:camera_controller
```

Each task binary creates the parameter struct (with any instance-level overrides), initializes internal state, then calls `on_init` and `on_update`.

## Step 5 — Run the unit tests

Each runnable has an auto-generated GTest unit test that verifies `on_init()` and `on_update()` return `Success` with default-initialized parameters and internal state.

Run all unit tests:

```bash
bazel test //src/...
```

Run a single runnable's unit test:

```bash
bazel test //src/adas/camera_controller:unit_test
```

Unit tests are located at `src/adas/<runnable>/test/unit_test.cpp` and use a [GTest](https://github.com/google/googletest) fixture with two test cases per runnable:

| Test case | Validates |
|---|---|
| `OnInitReturnsSuccess` | `on_init()` returns `score::base::ErrorCode::Success()` |
| `OnUpdateReturnsSuccess` | `on_update()` returns `score::base::ErrorCode::Success()` |

## Project structure

```
test-example/
├── graph_yaml/
│   ├── system_manifest.yaml      # Entry point — references all YAML files
│   ├── archetypes.yaml            # Runnable definitions (name, type, I/O ports)
│   ├── interfaces.yaml            # Shared data types (structs, enums, constexprs)
│   ├── instances.yaml             # Runnable instances with parameter overrides
│   ├── parameters/                # Per-runnable parameter definitions
│   └── internal_states/           # Per-runnable internal state definitions
├── src/                           # User-owned code (copied from src-gen, Step 2)
│   └── adas/
│       └── <runnable>/
│           ├── include/           # Runnable header (.hpp)
│           ├── src/               # Runnable source (.cpp)
│           ├── test/              # Unit test (unit_test.cpp)
│           └── BUILD.bazel        # cc_library + cc_test
├── src-gen/                       # Fully generated (never edited by user)
│   ├── interfaces/                # Interface headers
│   ├── parameter/                 # Parameter struct headers
│   ├── internal_state/            # Internal state struct headers
│   └── mini_adas/                 # Task binaries
│       ├── src/                   # Task main() files
│       └── BUILD.bazel            # cc_binary targets
├── MODULE.bazel                   # Generated Bazel module
└── BUILD.bazel
```
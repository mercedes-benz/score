# score-runtime

C++ runtime libraries for the score middleware framework. These libraries provide
the foundation that score-gen's backend generates code against.

## Building

```bash
cd score-runtime
bazel build //...
```

## Testing

```bash
bazel test //...
```

## Project structure

```
score-runtime/
├── include/score/     # Public headers
│   └── version.hpp
├── src/               # Implementation files
│   └── version.cpp
└── test/              # Unit tests (Google Test)
    └── version_test.cpp
```

## Usage from other Bazel projects

```python
# In your WORKSPACE or MODULE.bazel, reference score-runtime:
local_repository(
    name = "score-runtime",
    path = "../score-runtime",
)

# In BUILD.bazel:
cc_binary(
    name = "my_app",
    deps = ["@score-runtime//:score-runtime"],
)
```

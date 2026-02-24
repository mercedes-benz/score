# **Generic middleware-agnostic software atomic component container example**

We aim at demonstrating how a code generator would drastically reduce
the amount of "glue code" needed for the middleware and contribute to
enhance safety.

## **System Modeling Language**

The proposed System Modeling Language is yaml-based.  The directory
_systemyaml_ contains

- the description of the key concepts and supported features ([Link Text](./systemyaml/yaml_reference.md));
- the set of _json_ schema used to validate the yaml files structure.


## **Examples**

The example directory contains ([Link Text](./exmaples/README.md))

1. a set of yaml files (*graph_yaml*) that describes a mini adas system,
which would be the unique entry point for the application developer to
describe the system execution configuration. 

2. an example of generated code (*src_gen*).

The code was generated automatically with a custom code generator
written in Rust.  The custom generator is not provided. For a code
generator to be fully workable, S-core specific dependencies should be
integrated, e.g. an execution manager, a logging system, core
container libraries, and more. For the purpose of demonstration, the
resulting output is only provided as an example.

The second entry point for the developer to inject the algorithms is
pointed out in the code itself.

The generated code expects a build system based on Bazel. The unit
test code is based on GTest.

## **The Transformation Pipeline**
```
YAML System Definition → Code Generator → Generated C++ Code → Executable System
      (Human)              (SCOREGen)                            (S-core Runtime)
                                                            (compatible with FEO)
```

The Code Generator and Generated C++ Code are specific for
S-core execution model (FEO).

## **Conclusion**

To conclude, this example is not a fully working example since it
would need customization with S-core specific dependencies. It aims
only at demonstrating the contribution of a code generator to

1. the separation of concern between the middleware layer and the
application developer;

2. ensure an easy maintenance of the code;

3. ensure the portability of the code as the code generator would
transparently be transformed to integrate any new communication layer
or execution model, at no cost for the application developer;

4. enforce good coding practices, as the same certified
patterns are automatically enforced.


# System Models

System models are human-writable and machine-readable descriptions of graph algorithms. The modeled graphs
consist of dependent node archetypes that can be instantiated with different communication connections and parameters
to realize a given system. The primary intended operational domain for modeling a system with this language is in 
automotive ADAS/AD perception and planning systems, but the general concepts are also applicable in other domains 
and industries involving hard real-time computation constraints in automation systems.

## General syntax notes
The system modeling files use YAML as the underlying syntax. The files are validated against typed schemas
described in Rust as part of their parsing logic, as the system modeling language is not a free-form 
YAML-derivative language.

### Modeling language supported data types
Currently the following type specifiers are considered "built-in" types and are supported in terms of generating
simple code:
| Type | Description |
| ---- | ----------- |
| `uint8_t` | unsigned 8-bit integer |
| `uint16_t` | unsigned 16-bit integer |
| `uint32_t` | unsigned 32-bit integer |
| `uint64_t` | unsigned 64-bit integer |
| `int8_t` | signed 8-bit integer |
| `int16_t` | signed 16-bit integer |
| `int32_t` | signed 32-bit integer |
| `int64_t` | signed 64-bit integer |
| `bool` | boolean (assumed to be 8-bit) |
| `float` | floating point number (assumed to be 32-bit). NOTE: to be replaced by float32_t |
| `double` | floating point number (assumed to be 64-bit). NOTE: to be replaced by float64_t |
| `float32_t` | not yet properly supported  |
| `float64_t` | not yet properly supported |
| `size_t` | allowed, but not encouraged. Support dependent on back-end implementation. |
| `int` | allowed, but not encouraged. Support dependent on back-end implementation. |
---

Additionally, the following patterns are special cases in the YAML interpreter library:
| Pattern | Description |
| ------- | ----------- |
| `string` | A fixed-capacity string; this will be translated into an `FixedString<4096>` in C++. You can specify another capacity in the `capacity` field. |
| `typename[length]` | Creates a fixed-length array of `typename` elements, e.g. `uint8_t[64]`. |
| `vector<typename, capacity>` | Creates an `FixedVector`, i.e. a variable-length array, of maximum capacity `capacity`. |
| `nanoseconds` | A duration with nanosecond precision. The behavior is back-end implementation-specific. |
| `microseconds` | A duration with microsecond precision. The behavior is back-end implementation-specific. |
| `milliseconds` | A duration with millisecond precision. The behavior is back-end implementation-specific. |
| `seconds`  | A duration with second precision. The behavior is back-end implementation-specific. |
| `time_point` | An epoch-like description of a point in time. The behavior is back-end implementation-specific. |
---

## Common types
| Type | Description |
| ---- | ----------- |
| Namespace | Namespace string in C++ style notation (e. g. `name::space`) |
| StringRepresentedValue | Can be a string, integral, floating point or boolean, depending on the context. |
| Topic | Topic name string. Topic names are expected to follow the following pattern: `<output_namespace_snake_case>:<output_archetype_unique_name>.<output_instance_unique_name>:<output_unique_name>` |
---

## The system manifest

System models are written in a bespoke but simple description language based on the syntax of YAML. Each System
Model consists of 3 types of files, namely:
* Interface Lists
* Runnable Archetype Lists,
* Runnable Instance Lists

Additionally, for signal gateways or special tools that are not part of the computation graph, an optional System
Model file type Bridge List is used.

Within a given system model, there can be any number of the above greater than zero. All the modeling files need to be
listed in a system manifest, typically called `system_manifest.yaml` for automatic discoverability. The paths described
in the manifest must currently be *relative* to the parent directory that contains the `system_manifest.yaml` file.
This may change in the future to accommodate combining multiple sources of system modeling files.

The system manifest YAML file follows the following structure:

    system_name: "my_system"

    interfaces:
      - "my_datatypes.yaml"
      - "some_other_datatypes.yaml"

    archetypes:
      - "my_archetypes.yaml"
      - "some_other_archetypes.yaml"

    instances:
       - "my_graph_description.yaml"

    bridges:
       - "my_signal_gateways.yaml"

| Key | Type | Description |
| --- | ---- | ----------- |
| **system_name** | string | A name for the whole system model. |
| **interfaces** |  List of string | List of paths to interface definition files. Interface definition files describe the message data types used in the modeled system's communication. Paths are expected to be relative to the system manifest or absolute. |
| **archetypes** |  List of string | List of paths to archetype definition files. Archetype defintion files describe the software units to be deployed in the system. Paths are expected to be relative to the system manifest file or absolute. |
| **instances** |  List of string | List of paths to instance definition files. Instance definition files describe how instances of the modeled archetypes should be deployed at runtime and how the data flows through the system. The total data flow and execution graph can be modeled in one single file or it can be split up into any number of subgraph definitions. Communication interface connections in these files can reference topics from other files, but care needs to be taken to avoid typos. Paths are expected to be relative to the system manifest file or absolute. |
| **bridges** |  List of string (optional) | List of paths to bridge definition files. Bridge definition files list communication bridges (e. g. gateways) that are part of the system. Some pre-defined bridge types can be auto generated. Paths are expected to be relative to the system manifest file or absolute. |
---


The code generator works by parsing the manifest file first, and then every file it references. While the
parsing is happening file-by-file, the code generator builds up an in-memory intermediate representation of the
resulting system. Once the parsing is complete, the code generator does some integrity checks of the resulting graph
representation. If the tests pass, it starts generating the code according to the model and the user-provided
command line options.

## Writing Interface List YAML files

Each file describing a collection of interface data types is known as an Interface List.
Interface Lists are used to describe the contents of messages to be passed between Runnables,
providing a YAML-based Interface Description Language, similar to e.g. ROSIDL or ARXML type definitions.

Interface List files have the following general structure:
```yaml
namespace: my::name::space
structs:
  - name: MyMessageTypeName
    top_level: true
    members:
    - name: my_variable_name
        type: uint32_t

  - name: MyOtherMessageTypeName
    top_level: true
    members:
    - name: my_other_variable_name
        type: uint8_t

enums:
  - name: MyEnumClassName
    base: uint8_t
    values:
    - name: my_first_entry
    - name: my_second_entry
        value: 0x5

constexprs:
  - name: MyConst
    type: uint32_t
    value: 128

  [...]
```
The types defined in an Interface List file inherit the namespace of the Interface List. You can have any number of
Interface List files in a system model.

| Key | Type | Description |
| --- | ---- | ----------- |
| **namespace** | Namespace | Namespace all structs, enums and constexprs declared in this file will be part of. |
| **structs** |  List of InterfaceStructDefinition (optional) | List of `InterfaceStructDefinition` YAML objects, each describing one interface struct. In C++ these will be generated as `class`. |
| **enums** |  List of EnumDefinition (optional) | List of `EnumDefinition` YAML objects, each describing one enum. In C++ these will be generated as `enum class`. |
| **constexprs** |  List of StructMemberDefinition (optional) | List of `StructMemberDefinition` YAML objects, each describing once constexpr. These constexprs will be generated on namespace scope in C++. |
---


### The InterfaceStructDefinition syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **name** | string | Used to uniquely identify the data type both in all YAML files fed to the code generator and to generate unique type names for generated code. |
| **top_level** | boolean | Indicates whether or not the described data structure is a message type level or sub-component level data structure. Top level data structures can be created using other data structures modeled in YAML, that generally would not be message types themselves, i.e. not top-level. |
| **members** |  List of StructMemberDefinition | List of `StructMemberDefinition` YAML objects. If no members are modeled, an empty named data structure will be produced. |
| **header_files** |  List of string (optional) | Allows to provide 3rd party header inclusion paths to the code generator in case your types include 3rd party library types.  It is advised to minimize use of types from 3rd party libraries though as this will prevent auto-generation of gateways, which is planned for future releases, from working out of the box. It also introduces risk of mistakes by means of introducing types that depend on pointers to random memory locations, which will not work between processes. |
---


### The StructMemberDefinition syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **name** | string | Equivalent to the name of a struct member (field). |
| **type** | string | Type of the member variable. This is currently a free-form field that gets directly used by the code generator as a C++ type name. YAML-modeled data types that are defined in other Interface List files need to be referenced with their fully qualified namespace; types defined in the same file can be defined without the namespace qualification. This field can also be used to reference arbitrary types (e.g. from external libraries), but assignment operations and other code cannot be generated for them. If a data type not defined in YAML is referenced here, the code generator will issue warning but not consider it a hard error. |
| **default** | StringRepresentedValue (optional) | Is used for default-initialization of the member as part of the generated code type declaration. Default: language specific |
| **constraints** | StructMemberValueConstraint (optional) | Not currently used. Optional. |
| **constexpr** | boolean (optional) | For standalone `constexprs` the default is true, for others the default is false. When it is true, the member will be defined as `constexpr` with the value specified in the `value` field. |
| **value** | StringRepresentedValue (optional) | If this is a `constexpr`, the value it should have. Ignored otherwise. |
| **capacity** | 32-bit signed integer (optional) | Capacity of a fixes size container (e. g. a string or a vector). |
---


### The EnumDefinition syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **name** | string | Name of the enum, used to uniquely identify the data type both in all YAML files fed to the code generator and to generate unique type names for generated code. |
| **base** | string (optional) | Allows to specify the enum class base type in C++. Default: compiler specific |
| **values** |  List of EnumEntry | A YAML list of `EnumEntry` objects. |
---


### The EnumEntry syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **name** | string | The name of the enum entry (e. g. `kMyEnumEntry`). |
| **value** | string or 32-bit signed integer (optional) | A specific value associated with the enum entry. |
---


## Writing Runnable Archetype List YAML files

Each file describing a collection of Runnable Archetypes is a Runnable Archetype List. Each Runnable Archetype List
needs to specify a namespace. Runnable Archetype Lists are used to describe existing (or intended) software unit
implementations-in-code that follow the Runnable API specification. Runnable Archetype Descriptions are used to model
the characteristics of Runnables. Each system model needs to include at least one Runnable Archetype List.

A Runnable Archetype List generally looks something like this:
```yaml
namespace: ExampleRunnableGroup
runnables:
  - unique_name: MovingAverageCounterNode  # Runnable Archetype name
    min_interval_us: 50000    # Limit execution to once every 50'000 microseconds
    max_interval_us: 100000   # Execute at least every 100'000 microseconds
    internal_state_header: "MyLastFiveInputs.yaml"  # The internal state data structure is defined in this file
    build_information:
      - bazel_target: "//my_project/src/ExampleRunnableGroup/MovingAverageCounterNode:MovingAverageCounterNode"
    inputs:
      - unique_name: input_number
        type_name: example::Number
        n_samples_min: 1  # This runnable needs at least 1 instance of this input as a data input condition trigger
        n_samples_max: 1  # This runnable can only handle up to 1 instance of this input per execution cycle
    outputs:
      - unique_name: moving_average
        type_name: example::Number
        n_samples_max: 1  # This runnable will output a maximum of 1 instances of this output per execution cycle
        n_slots: 2  # The shared-memory interface will have two slots (one writer + one reader == 2 actors)
        memory_backend: PosixShm # The shared memory backend this output will be stored in
  [...]
```

The `runnables` list consists of YAML entries interpreted into RunnableArchetypeDescription objects using the
Rust-based schemas.

The `namespace` is used for logical grouping of functionality within the system. Practical examples: "sensors",
"perception", "planning". Namespace components are separated by two colons (::)

| Key | Type | Description |
| --- | ---- | ----------- |
| **namespace** | Namespace | Namespace all runnables defined in this YAML file should be part of. The notation is C++ style, e. g. `my::namespace`. |
| **runnables** |  List of RunnableArchetypeDescription | List of `RunnableArchetypeDescription` defining Runnable Archetypes. |
---


### The RunnableArchetypeDescription syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **unique_name** | string | A unique shorthand or name for the specific Runnable implementation-in-code. NOTE: Must be PascalCased! |
| **namespace** | Namespace (optional) | **DEPRECATED** Use the global `namespace` field instead. Used to impact the directory tree for generated runnable skeletons. |
| **runnabletype** | string | Currently compulsory but not actually used. Intended to describe the compute unit used by the runnable, e. g. `CPU` or `GPU`. |
| **wcet_us** | 64-bit signed integer | Indicates the Worst-Case Execution Time for the Runnable's `onUpdate()` function. Unit: microseconds |
| **min_interval_us** | 64-bit signed integer (optional) | Limits how often a Runnable's `onUpdate()` function gets triggered, even if all other conditions were met. Default: no minimum interval. Unit: microseconds |
| **max_interval_us** | 64-bit signed integer (optional) | Used to force a Runnable's `onUpdate()` functon to get triggered at least this often. Default: never. Unit: microseconds |
| **parameter_header** | string (optional) | Path relative to the YAML directory root, to a data structure description YAML file describing the Runnable's constant runtime parameters, such as calibration settings or which sensor device to use. Default: no parameters |
| **internal_state_header** | string (optional) | Path relative to the YAML directory root, to a data structure description YAML file describing a container data structure for internal state variables and data structures necessary for reproducibility in input-output determinism for stateful algorithms. Default: no internal state |
| **inputs** |  List of RunnableInputDescription (optional) | List of input data interfaces expected by a Runnable Archetype implementation in the form of `RunnableInputDescription`s. |
| **outputs** |  List of RunnableOutputDescription (optional) | List of output data interfaces expected by a Runnable Archetype implementation in the form of `RunnableOutputDescription`s. |
| **build_information** | BuildInformation | The fully qualified absolute Bazel target label of the Runnable Archetype implementation (library) in the Bazel workspace. This is used for auto-generating the project Bazel BUILD files. |
---


If none of `min_interval_us` and `max_interval_us` are specified, then the runnable will only be triggered when its
data input conditions are met (see RunnableInpuDescription).

#### The RunnableInputDescription syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **unique_name** | string | A unique (for a Runnable Archetype) identifier for the modeled input interface. This also acts as the locally accessible name of the interface in the Runnable's in-code implementation. |
| **type_name** | string | Message type (see `InterfaceStructDefinition`) to be used for this input interface. This type name has to be identical to the type of the corresponding output that's connected to this input in the instance definition. |
| **n_samples_max** | 16-bit unsigned integer | Maximum number of input samples that should be queued for this input. All of the queued samples are made available to one iteration of the runnable's `onUpdate()`. This is typically useful if not all inputs arrive at the same frequency. |
| **n_samples_min** | 16-bit unsigned integer (optional) | Minimum number of inputs samples that should be queued for this input before the runnable's `onUpdate()` is triggered (unless `max_interval_us` is reached). A value of zero means this input is not taken into consideration when deciding if the runnable is ready to be triggered. Default: 0 |
| **policy** | BatchPolicy (optional) | Either `kLastN` or `kNewestN`. `kLastN` causes the input queue for this input to be flushed when the runnable's `onUpdate()` is triggered. `kNewestN` retains the newest `n_samples_max` samples in the input queue over multiple triggers of the runnable. **IMPORTANT:** `kNewestN` in combination with `n_samples_min` can cause the runnable to be triggered constantly. It can be useful in combination with other inputs which also define `n_samples_min` though. Default: `kLastN` |
---


#### The RunnableOutputDescription syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **unique_name** | string | Unique (for a Runnable Archetype) identifier for the modeled output interface. This also acts as the locally accessible name fo the interface in the Runnable code. |
| **type_name** | string | Message type (see InterfaceStructDefinition) to be used for this output interface. |
| **n_samples_max** | 16-bit unsigned integer | Maximum number of outputs via this interface that the Runnable implementation will produce in one `onUpdate()` cycle. |
| **n_slots** | 16-bit unsigned integer (optional) | Number of in-memory slots used for the single-writer-multiple-reader zero-copy interface. For complex systems, this number requires some calculation, but for a single-writer-single-reader use-case this can safely be set to 2. Default: auto-calculated |
| **memory_backend** | MemoryBackend (optional) | Defines the memory backend used for sample distribution, either `kPosixShm` or `kPcieShm`. Default: `kPosixShm` |
---


## Writing Runnable Instance List YAML files (or rather, Graph Descriptions)

Runnable Instance Lists are YAML files that describe the instantiation of Runnable Archetypes into an actual system,
where the Runnable Instances form a communication graph.

A Runnable Instance List looks something like this:
```yaml
namespace: ParkingImageProcessing  # Deployment group name, freeform
instances:
  - archetype_reference: CameraController  # The name of the Runnable Archetype to instantiate
    unique_name: camera0    # A unique identifier of this Runnable Archetype Instance
    parameter_overrides:    # The list of parameter overrides specific to this archetype instance
      - name: camera_index  # Override the default value of the archetype's parameter field camera_index
        value: 0            # ...with the value 0
    outputs:                # The list of archetype outputs to instantiate and connect to other entities
      - output_reference: camera_out  # Instantiate the archetype's output interface camera_out with the below topic
        topic_name: ParkingImageProcessing:ParkingCameras.camera0:image
    execution_params:
    autostart: true  # Always start this component as part of system bring-up
    autorestart: true  # Always restart this component if it exits during the system runtime

  - archetype_reference: CameraController
    unique_name: camera1
    parameter_overrides:
      - name: camera_index
        value: 1
    outputs:
      - output_reference: camera_out
        topic_name: ParkingImageProcessing:ParkingCameras.camera1:image
    execution_params:
    autostart: true
    autorestart: true

  - archetype_reference: ImageStitcher
    unique_name: parking_stitcher
    inputs:
      - input_reference: camera_in_left
        topic_name: ParkingImageProcessing:ParkingCameras.camera0:image
      - input_reference: camera_in_right
        topic_name: ParkingImageProcessing:ParkingCameras.camera1:image
    outputs:
      - output_reference: stitched_image
        topic_name: ParkingImageProcessing:ImageStitcher.parking_stitcher:image_stitched
    execution_params:
    autostart: true
    autorestart: true
  [...]
```

The above example describes two instances of a Runnable Archetype called CameraController, each for a different
physical camera (identified by indices 0 and 1 respectively). The outputs modeled in these instances are then
connected to an instance of the Runnable Archetype called ImageStitcher, that has two modeled inputs, namely
camera_in_left and camera_in_right. The instance of the image stitcher then outputs a stitched image under the topic
`ParkingImnageProcessing:ImageStitching.parking_stitcher:image_stitched.

| Key | Type | Description |
| --- | ---- | ----------- |
| **namespace** | Namespace | Namespace of the Runnable Instances for logical grouping. Fully qualified output topics of Runnable Instances will use this as the namespace prefix. The generated deployment code directory structure will reflect this namespacing. The notation is C++ style, e. g. `my::namespace`) |
| **instances** |  List of RunnableInstanceDescription | List of `RunnableInstanceDescription` describing the instanced of Runnable Archetypes in the system. |
---


### The RunnableInstanceDescription syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **archetype_reference** | string | Identifier referencing the Runnable Archetype of this instance, modeled in one of the Runnable Archetype List YAML files. |
| **unique_name** | string | Unique identifier or name of the specific instance of the Runnable Archetype. This will also be the name of the Runnable Instance executable produced by the code generator. |
| **period_ns** | 64-bit signed integer (optional) | Currently unused. |
| **runtime_ns** | 64-bit signed integer (optional) | Currently unused. |
| **wcet_override_us** | 64-bit signed integer (optional) | Overrides the WCET specified for the Runnable Archetype. This can be useful if a specific instance has a different WCET. Default: unchanged. Unit: microseconds |
| **min_interval_override_us** | 64-bit signed integer (optional) | Overrides the `min_interval_us` for this particular instance of the Runnable Archetype. Default: unchanged. Unit: microseconds |
| **max_interval_override_us** | 64-bit signed integer (optional) | Overrides the `max_interval_us` for this particular instance of the Runnable Archetype. Default: unchanged. Unit: microseconds |
| **parameter_overrides** |  List of ParameterOverrideDescription (optional) | Instance-specific values for the parameters modeled for the underlying Runnable Archetype, overriding the default values. These can be overwritten once more at startup-time with parameter files. |
| **inputs** |  List of InstanceInputChannel (optional) | List of `InstanceInputChannel`s. Identifies uniquely which of the modeled Runnable Archetype's inputs are mapped to which topic name. All inputs of the archetype must be mapped. |
| **outputs** |  List of InstanceOutputChannel (optional) | List of `InstanceOutputChannel`s. Identifies uniquely which of the modeled Runnable Archetype's outputs are mapped to which topic name. All outputs of the archetype must be mapped. |
| **execution_params** | ExecutionParameters (optional) | `ExecutionParameters` structure controlling the execution of the Runnable Instance (e. g. whether the instance should be stared by the execution manager). Default: autostart / no auto-restart |
---


#### The ParameterOverrideDescription syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **name** | string | Reference to a field in the YAML-modeled parameter data structure of the Runnable Archetype (if any is defined). |
| **value** | StringRepresentedValue | Value to override the YAML-modeled parameter's default value with. NOTE: overriding of array or struct type parameter fields is not currently supported. |
---


#### The InstanceInputChannel syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **input_reference** | string | Reference to a YAML-modeled input name of the underlying Runnable Archetype. |
| **topic_name** | Topic | Fully-qualified topic name that must match the output of another Runnable Instance modeled in the same collection of YAML files. Fully-qualified topic names follow the pattern `<output_namespace_snake_case>:<output_archetype_unique_name>.<output_instance_unique_name>:<output_unique_name>`. |
---


#### The InstanceOutputChannel syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **output_reference** | string | Reference to a YAML-modeled output name of the underlying Runnable Archetype. |
| **topic_name** | Topic | Fully-qualified topic name this output shall be available under. Fully-qualified topic names follow the pattern `<output_namespace_snake_case>:<output_archetype_unique_name>.<output_instance_unique_name>:<output_unique_name>`. |
---


#### The ExecutionParameters syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **autostart** | boolean | Controls whether the instance shall be started by the Execution Manager. Default: `true` |
| **autorestart** | boolean (optional) | Controls whether the instance shall be automatically re-started if the process exits for any reason. Default: `true` |
---


## Writing Bridge List YAML files
```yaml
namespace: ExampleDeploymentGroup

bridges:
  - type_reference: "Recorder"  # Special reserved name
    unique_name: "recorder"  # The compiled executable name
    min_interval_us: 10000  # Limit CPU time provision to once every 10'000 microseconds
    inputs: [  # The list of input topics to support recording of
        "ExampleDeploymentGroup:ExampleServiceB.runnableinstance1:ExampleServiceBTopic",
    ]
    execution_params:
        autostart: false
        autorestart: false

  - type_reference: "Player"  # Special reserved name
    unique_name: "player"  # The compiled executable name
    outputs: [  # The list of output topics to support playback of
        "ExampleDeploymentGroup:ExampleRootService0.RootInstanceA:ExampleRootTopic0"
    ]
    execution_params:
        autostart: true
        autorestart: true

  - type_reference: "ROS2Gateway"  # Special reserved name
    unique_name: "rosgateway"  # The compiled executable name
      [...]
```

Bridge Lists are used to model Bridges, which is the term used here to describe signal gateways and other
graph-interacting components that interact with the middleware space but are not fully managed by the middleware.

| Key | Type | Description |
| --- | ---- | ----------- |
| **namespace** | Namespace | Namespace of bridge instances for logical grouping. Currently only used in internal implementation details for increasing unique identifiability of the Bridge. |
| **bridges** |  List of BridgeDescription | List of `BridgeDescription`s listing bridge definitions. |
---


### The BridgeDescription syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **type_reference** | BridgeType | Type of the bridge. Currently only built-in Bridge targets are supported. These are `Player`, `Recorder` and `ROS2Gateway` (case-sensitive). Support for custom Bridges will follow soon. |
| **unique_name** | string | A unique name for the bridge instance. This will also be the name of the bridge executable produced by the code generator. |
| **min_interval_us** | 64-bit signed integer (optional) | Minimum interval with which the Bridge will be triggered to process data. This is used to limit CPU time provision and defaults to zero. If one of the Bridge's input queues is full this interval is overridden. Default: 0. Unit: microseconds |
| **configs** |  List of string (optional) | List of bridge type specific config files needed for code generation. Currently only the `ROS2Gateway` can use of config files to allow custom type mappings. Default: none |
| **inputs** |  List of BridgeConnection (optional) | List of fully-qualified topic names of topics that shall be received by the bridge. `*` wildcards are supported here. E. g. `*` configures all topics available in the system to be an input to the bridge. Optional. |
| **outputs** |  List of BridgeConnection (optional) | List of either fully-qualified topic names the bridge outputs (with wildcard support) or `BridgeConnectionDefinition`s defining the outputs. The simplified form only specifying the output topic names can only be used if the bridge is not the only producer of these topics (e. g. for a player bridge). Default: none |
| **parameter_header** | string (optional) | Path relative to the YAML directory root, to a data structure description YAML file describing the Bridge's constant runtime parameters, such as calibration settings or which sensor device to use. Default: no parameters |
| **parameter_overrides** |  List of ParameterOverrideDescription (optional) | Instance-specific values for the parameters modeled for the underlying Bridge, overriding the default values. These can be overwritten once more at startup-time with parameter files. |
| **execution_params** | ExecutionParameters (optional) | Parameters controlling the execution of the instance (RunnableInstanceDescription / ExecutionParameters). Optional. |
| **runnables_list** |  List of PortName (optional) | List of runnables in the form `<namespace>:<service_name>:<instance_name>` (`service_name` is typically the archetype name) this bridge should record/replay parameters, internal states, input samples and jobs for (only relevant for recorder and player bridges). |
---


#### Input/output lists with wildcard support
| topic_name: | Description |
| ----------- |-------------|
| camerapipeline:cameracontroller.cam1:IMAGE | unambiguous, fully qualified | camerapipeline:cameracontroller.cam1:IMAGE |
| camerapipeline:* | Match every topic in the `camerapipeline` deployment namespace, i.e. camerapipeline:*.*:* |
| *  | equivalent to *:*.*:*, so all defined topic_names will match |
| *:IMAGE | equivalent to *:*.*:IMAGE |
| camerapipeline:cameracontroller.*:IMAGE | this will match all topics called 'IMAGE' from all service instances of the service 'camerapipeline:cameracontroller'.  |
---

#### The BridgeConnectionDefinition syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **topic** | Topic | Fully-qualified topic name of the connection. Fully-qualified topic names follow the pattern `<output_namespace_snake_case>:<output_archetype_unique_name>.<output_instance_unique_name>:<output_unique_name>`. |
| **type_name** | string (optional) | Message type (see `InterfaceStructDefinition`) to be used for this interface. This type name has to be identical to the type of the corresponding input/output that's connected to this output/input. Only required if the bridge is the only producer of the output. |
| **n_slots** | 16-bit unsigned integer (optional) | Number of in-memory slots used for the single-writer-multiple-reader zero-copy interface. This number is only relevant if the this connection definition defines an output. For complex systems, this number requires some calculation, but for a single-writer-single-reader use-case this can safely be set to 2. Default: auto-calculated. |
---


## Writing a Deployment Configuration YAML file (optional)
```yaml
deployment_configuration:
  - instance_unique_name: <unique_name>
    cpu_affinities: [ <values>]
    process_scheduling:
        scheduling_policy: <scheduling option>
        scheduling_priority: <value>
  [...]
```

The Deployment Configuration YAML file is optional and specifies deployment-specific modifications of the
code-generated system components' behavior, for example to configure the priority or core assignment of a process.

If a deployment configuration is given for an instance, all fields must be provided.

| Key | Type | Description |
| --- | ---- | ----------- |
| **deployment_configuration** |  List of DeploymentConfigurationConfig | List of `DeploymentConfigurationConfig`s. Runnable Instances for which no deployment configuration is provided will run with the operating system defaults. |
---


### The DeploymentConfiguration syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **instance_unique_name** | string | The unique_name used for any generated Runnable Instance or bridge this configuration is for. |
| **cpu_affinities** |  List of 16-bit unsigned integer | Array of (zero-based) indices of CPU cores available to the referred Runnable Instance in the range `[0, <max_num_cores>-1]`. |
| **process_scheduling** | DeploymentConfigurationProcessSchedulingConfig | Realtime scheduling parameters to be set for the referred Runnable Instance. NOTE: The execution manager may need elevated permissions to be able to apply those parameters. |
---


### The ProcessScheduling syntax
| Key | Type | Description |
| --- | ---- | ----------- |
| **scheduling_policy** | SchedulingPolicy | Scheduling policy the operating system scheduler shall use. One of: `kOther` (translates to SCHED_OTHER), `kFifo` (translates to SCHED_FIFO) or `kRoundRobin` (translates to SCHED_RR). Default: `kOther` |
| **scheduling_priority** | 16-bit signed integer | Scheduling priority the operating system scheduler shall apply. Values ranges are OS specific. |
---


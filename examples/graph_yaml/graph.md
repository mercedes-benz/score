# Mini adas system - Simplified sequential architecture

## Graph representation

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                               MINI-ADAS SYSTEM                                       │
│                           (Sequential Pipeline Architecture)                        │
│                                                                                     │
│  ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐              │
│  │   CAMERA        │────▶│   NEURAL NET    │────▶│ BRAKE CONTROLLER│              │
│  │  (front_camera) │     │(object_detector)│     │(emergency_brake)│              │
│  │                 │     │                 │     │                 │              │
│  │ WCET: 5ms       │     │ WCET: 25ms      │     │ WCET: 2ms       │              │
│  │ Period: 33ms    │     │ Period: 30ms    │     │ Period: 10ms    │              │
│  └─────────────────┘     └─────────────────┘     └─────────────────┘              │
│                                   │                                                │
│                                   ▼                                                │
│                          ┌─────────────────┐                                      │
│                          │ TRAFFIC SIGN    │                                      │
│                          │  RECOGNITION    │                                      │
│                          │(traffic_sign_   │                                      │
│                          │   controller)   │                                      │
│                          │                 │                                      │
│                          │ WCET: 10ms      │                                      │
│                          │ Period: 100ms   │                                      │
│                          └─────────────────┘                                      │
│           │                        │                        │                     │
│           │                        │                        │                     │
│           ▼                        ▼                        ▼                     │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                          SYSTEM LOGGER                                      │   │
│  │                     (system_data_logger)                                    │   │
│  │                                                                             │   │
│  │  • Logs camera, detections, brake commands, and traffic signs              │   │
│  │  • WCET: 1ms, Period: 5ms                                                  │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
│           │                        │                        │                     │
│           ▼                        ▼                        ▼                     │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                      PERFORMANCE TRACER                                     │   │
│  │                    (system_perf_monitor)                                    │   │
│  │                                                                             │   │
│  │  • Monitors CPU, memory, timing of all components including TSR            │   │
│  │  • WCET: 0.5ms, Period: 1ms                                               │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                     │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

## Detailed Component Analysis

### 1. **Camera Controller** (front_camera) - SOURCE
- **Function**: Captures camera frames
- **Output**: `raw_frames` topic → CameraFrame data
- **Timing**: WCET 5ms, Period 33.3ms (30 FPS)
- **Dependencies**: None (source component)
- **Consumers**: Neural Network + Logger + Tracer

### 2. **Neural Network** (object_detector) - INTERMEDIATE
- **Function**: Object detection inference (including traffic signs)
- **Input**: `raw_frames` topic ← Camera Controller
- **Output**: `detected_objects` topic → ObjectDetections data
- **Timing**: WCET 25ms, Period 30ms
- **Dependencies**: Camera Controller
- **Consumers**: Brake Controller + Traffic Sign Recognition + Logger + Tracer

### 3. **Traffic Sign Recognition** (traffic_sign_controller) - SEQUENTIAL PROCESSOR
- **Function**: Extracts traffic sign information from detected objects
- **Input**: `detected_objects` topic ← Neural Network
- **Output**: `sign_commands` topic → TrafficSignCommand data
- **Timing**: WCET 10ms, Period 100ms (10 Hz - signs change slowly)
- **Dependencies**: Neural Network (sequential processing)
- **Logic**: 
  - Filters objects with ObjectClass::kTrafficSign
  - Performs detailed sign classification on detected regions
  - Extracts speed limits, warning types, regulatory information
  - Outputs structured traffic sign commands

### 4. **Brake Controller** (emergency_brake_ctrl) - SAFETY CRITICAL
- **Function**: Emergency braking decisions  
- **Input**: `detected_objects` topic ← Neural Network
- **Output**: `brake_commands` topic → BrakeCommand data
- **Timing**: WCET 2ms, Period 10ms (SAFETY CRITICAL!)
- **Dependencies**: Neural Network
- **Logic**: Processes vehicles, pedestrians, cyclists for collision detection

### 5. **System Logger** (system_data_logger) - MONITORING
- **Function**: Data logging and audit trail
- **Inputs**: 
  - `raw_frames` ← Camera Controller
  - `detected_objects` ← Neural Network
  - `brake_commands` ← Brake Controller  
  - `sign_commands` ← Traffic Sign Recognition
- **Output**: `log_entries` topic → LogEntry data
- **Timing**: WCET 1ms, Period 5ms
- **Dependencies**: Independent monitoring (subscribes to all)

### 6. **Performance Tracer** (system_perf_monitor) - MONITORING
- **Function**: Real-time performance monitoring
- **Inputs**:
  - `raw_frames` ← Camera Controller
  - `detected_objects` ← Neural Network
  - `brake_commands` ← Brake Controller
  - `sign_commands` ← Traffic Sign Recognition
- **Output**: `performance_traces` topic → PerformanceTrace data
- **Timing**: WCET 0.5ms, Period 1ms (highest frequency)
- **Dependencies**: Independent monitoring (subscribes to all)


## System Configuration

### **Core YAML Files**
- `system_manifest.yaml` - System definition entry point
- `interfaces.yaml` - Data types and message definitions
- `archetypes.yaml` - Component templates with timing constraints
- `instances.yaml` - Deployed component configuration
- `bridges.yaml` - External system interfaces (ROS2, recorder, player)

### **Parameter Files** (`parameters/`)
- `CameraControllerParameters.yaml` - Camera settings (resolution, framerate)
- `NeuralNetParameters.yaml` - AI model configuration  
- `TrafficSignParameters.yaml` - Sign detection thresholds
- `BrakeControllerParameters.yaml` - Safety system parameters
- `LoggerParameters.yaml` - Logging configuration
- `TracerParameters.yaml` - Performance monitoring settings

### **Internal State Files** (`internal_states/`)
Component private state definitions for each processing element.

## Topic Communication Details

### **Core Data Topics**
1. `raw_frames` - Camera → Neural Network, Logger, Tracer
2. `detected_objects` - Neural Network → Brake Controller, TSR, Logger, Tracer  
3. `brake_commands` - Brake Controller → Logger, Tracer
4. `sign_commands` - Traffic Sign Recognition → Logger, Tracer
5. `log_entries` - System Logger output
6. `performance_traces` - Performance Tracer output



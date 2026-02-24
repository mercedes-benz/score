# Mini ADAS example 

A comprehensive example showcasing a mini ADAS (Advanced Driver
Assistance System) with multiple components.

## System Overview

The system is a multi-components automotive safety system with:

### **Primary Safety Pipeline** (Critical)
```
Camera → Neural Network → Emergency Brake Controller
                           ↓
                    Traffic Sign Recognition
```

### **Independent Monitoring Services**
```
System Logger + Performance Tracer (monitor all components)
```

## Generated Dependency Graph

See `graph_yaml/graph.md` for detailed component relationships and data flow visualization.
This enhanced **Mini-ADAS** system extends the original autonomous braking example with:

### **Additional Components**
- Traffic Sign Recognition (parallel processing)
- Enhanced logging (4-input system logger)  
- Comprehensive monitoring (all-component tracer)

### **Advanced Architecture**
- Multi-modal sensor fusion (camera + neural net → TSR)
- Parallel processing pipelines
- Realistic automotive timing (fast safety, slower advisory)
- Production-ready data types (European traffic signs)

## **Generated Components**

The code generator output is generating C++ code for:

### **Core Processing Components**
- `CameraController/` - Camera sensor interface
- `NeuralNetInference/` - Object detection neural network
- `TrafficSignRecognition/` - Traffic sign detection and classification
- `BrakeController/` - Emergency braking logic

### **System Services**
- `SystemLogger/` - Data logging service
- `PerformanceTracer/` - Real-time performance monitoring

### **Build the System** 
```bash
# Build all components 
bazel build //src-gen/...
```

## Learning Objectives

This demo expect middleware fundamental concepts to be understood: 

- **Component Architecture**
- **Communication**
- **Execution manager**

This demo teaches key concepts:
1. the separation of concern between the middleware layer and the
application developer;

2. ensure an easy maintenance of the code;

3. ensure the portability of the code as the code generator would
transparently be transformed to integrate any new communication layer
or execution model, at no cost for the application developer;

4. enforce good coding practices, as the same certified
patterns are automatically enforced.


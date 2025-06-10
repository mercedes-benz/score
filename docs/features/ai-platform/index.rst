..
   # *******************************************************************************
   # Copyright (c) 2025 Contributors to the Eclipse Foundation
   #
   # See the NOTICE file(s) distributed with this work for additional
   # information regarding copyright ownership.
   #
   # This program and the accompanying materials are made available under the
   # terms of the Apache License Version 2.0 which is available at
   # https://www.apache.org/licenses/LICENSE-2.0
   #
   # SPDX-License-Identifier: Apache-2.0
   # *******************************************************************************

.. _aip_feature:

AI Platform
###########

.. document:: AI-Platform
   :id: doc__aipcs
   :status: draft
   :safety: ASIL_B
   :tags: feature_request


Feature flag
============

To activate this feature, use the following feature flag:

``experimental_aip``


Abstract
========

This feature request outlines the foundational requirements for integrating AI workloads into the S-CORE automotive platform,
with a particular emphasis on enabling inference capabilities across both QNX and Linux operating systems.
The primary goal is to provide support for ASIL-B compliant use cases on QNX through a thin,
vendor-agnostic abstraction layer for AI backends such as TensorRT or QNN.
For non-safety-critical applications, a standardized inference backend—such as ONNX Runtime—should be supported,
despite its current lack of compatibility with QNX.
Additionally, Generative AI (GenAI) related components shall be provided for Linux based systems.

The document proposes extending S-CORE components (e.g., FEO, Communication, Error Handling)
to support AI models natively, avoiding duplicate logic across software domains.
Furthermore, it introduces a scoped investigation into GPU shared memory (SHM) and
data pipelining mechanisms to abstract communication of GPU-resident objects.


Motivation
==========



Rationale
=========



Specification
=============

This feature request aims to extend the S-CORE platform to support AI workloads across both QNX and Linux environments,
enabling safe and efficient execution of models for both safety-critical and non-critical automotive functions.
The architectural concept focuses on a modular inference pipeline, supporting a unified abstraction for AI model execution,
backend integration, and GPU-based communication.

Operating System Support and ASIL Alignment
___________________________________________

The platform must support both Linux and QNX, with differing priorities and use-case profiles.
QNX is prioritized (Priority 1) due to its relevance in safety applications.
Linux support (Priority 2) primarily targets non-safety-critical applications, including Generative AI (GenAI) workloads.
Safety use cases will adhere to the constraints imposed by functional safety requirements,
whereas Linux allows for more flexible development.
All features available on QNX will also support Linux.


QNX
---

The figure below shows the high level scope for the QNX platform with a target of ASIL-B.
The two main components are the **Vendor Abstraction** (Backend Adapter) and the **Data Pipeline**.

.. image:: _assets/score-aip-qnx.drawio.svg
   :alt: AI Platform Architecture Overview QNX


Linux
-----

The next figure shows the scope of the Linux-based platform.
All components running on QNX shall also run on Linux - but not the other way around.
In addition to the QNX scope, GenAI related components like **MCP Server** a **Context API** included.

.. image:: _assets/score-aip-linux.drawio.svg
   :alt: AI Platform Architecture Overview Linux


Inference Backend Integration and Abstraction Layer
___________________________________________________

The idea of this component is to provide a lightweight and certifiable abstraction layer that decouples applications from vendor specific APIs.

To provide model execution capability, the system must support inference backends via a thin abstraction layer.
This layer will expose a unified interface to the upper layers of the stack while delegating execution to optimized
vendor runtimes underneath — such as TensorRT for NVIDIA, or QNN for Qualcomm-based systems.
For non-safety use cases, a standardized backend like ONNX Runtime should be supported to ensure portability and developer accessibility.
However, ONNX Runtime currently lacks QNX support - which will further be investigated.


Concept
-------

The diagram below illustrates the architecture of the AIP Sandbox.
It highlights how a unified Adapter Interface allows seamless integration with different hardware-dependent inference backends
(e.g., ONNX Runtime, TensorRT), as well as a mock backend for testing.
The IOUtils module handles preprocessing and input preparation.
Keeping IOUtils as a separate library helps isolate input handling logic from inference logic,
making it easier to test, reuse, and extend preprocessing across different models and backends.


.. uml::

   @startuml
   ' Define components
   component "Main Application" as A
   component "ModelAPI" as B
   component "Adapter Interface" as C
   component "ONNXAdapter\nONNX Runtime" as D1
   component "TRTAdapter\nTensorRT" as D2
   component "MockAdapter\nfor testing" as D3
   component "IOUtils / Preprocessor" as E
   component "Normalize" as F1
   component "Load .pb\nTest Data" as F2
   component "ONNX Runtime Lib" as G1
   component "TensorRT Engine" as G2
   component "Dummy Backend" as G3

   ' Define relationships
   A --> B
   B --> C
   C --> D1
   C --> D2
   C --> D3

   B --> E
   E --> F1
   E --> F2

   D1 --> G1
   D2 --> G2
   D3 ..> G3 : dummy
   @enduml


Key benefits of this concept include:

- Static backend selection at compile time ensures deterministic behavior and reduces runtime complexity.
- Clear separation of responsibilities (e.g., IOUtils vs inference adapters) supports modular safety analysis.
- MockAdapter enables early testing and CI validation without requiring hardware targets.
- Minimal and auditable abstractions make the system easier to verify and validate, especially when wrapping certified inference engines such as TensorRT (when used as a Safety Element out of Context, SEooC).

This structure allows isolating and certifying components independently, which is essential for scalable safety certification.


Adapter Class
-------------

The class diagram below shows the object-oriented structure of the Adapter system.
All backend adapters inherit from a shared abstract interface, ensuring consistent model loading and inference APIs across implementations.


.. uml::

   @startuml
   abstract class AdapterInterface {
      +loadModel(path): bool
      +infer(input, output): bool
   }

   class ONNXAdapter {
      +loadModel(path): bool
      +infer(input, output): bool
   }

   class TRTAdapter {
      +loadModel(path): bool
      +infer(input, output): bool
   }

   class MockAdapter {
      +loadModel(path): bool
      +infer(input, output): bool
   }

   AdapterInterface <|-- ONNXAdapter
   AdapterInterface <|-- TRTAdapter
   AdapterInterface <|-- MockAdapter
   @enduml


Backend Selection Mechanism
---------------------------

The following diagram shows how the backend implementation is selected at compile time via CMake flags.
Depending on the configuration, either the ONNX Runtime, TensorRT, or a mock adapter is compiled into the application.


.. uml::

   @startuml
   object "CMake Configuration" as A
   object "USE_ONNX / USE_MOCK_TRT / USE_TRT" as B
   object "ONNXAdapter enabled" as C
   object "MockAdapter enabled" as D
   object "TRTAdapter enabled" as E

   A --> B
   B --> C : USE_ONNX
   B --> D : USE_MOCK_TRT
   B --> E : USE_TRT
   @enduml


Data Pipelining and GPU Communication Abstraction
_________________________________________________

Many models—especially vision-based ones—depend on high-throughput data exchange with GPU memory.
To support efficient data flow, the architecture should provide a data pipelining layer that abstracts GPU-resident communication objects.
This may include:

- Shared memory buffers between producer (e.g., camera capture) and consumer (e.g., model runner)
- Zero-copy mechanisms to minimize CPU-GPU transfers
- Standardized data contracts for tensor formats and metadata

A key challenge here is observability: current S-CORE recording and instrumentation may not capture GPU-to-GPU data flows.
The architecture should address this gap by introducing pipeline tap points or mirrors to enable debugging and traceability.
A second challenge is the tight coupling of GPU memory object to vendor specific libraries.

Therefore, the exact scope and feasibilty of this component must be investigated in-depth by a future feature request.


.. image:: _assets/score-aip-abstraction.drawio.svg
   :alt: AI Platform Abstraction


S-CORE Integration: FEO, Communication, and Error Handling
__________________________________________________________

AI model execution should be integrated into existing S-CORE components—not implemented as a standalone subsystem.
This includes:

- FEO: Integration allows AI tasks to be scheduled and monitored like any other software component.
- Communication: Model inputs and outputs must seamlessly fit into the existing communication model.
- Error Handling: Faults and anomalies during inference (e.g., invalid input tensors, timeout, memory access issues) must be reported and handled using S-CORE’s diagnostics framework.
- Recording: Data between AI/ML nodes with GPU memory object should be recordable in the same manner as regular IPC communication.

This unified approach avoids fragmentation and ensures that AI models are treated as first-class citizens within the system.

GenAI
_____

The figure below shows an intial scope of components to be considered as part of the GenAI toolkit.

.. image:: _assets/score-aip-genai-overview.drawio.svg
   :alt: AI Platform GenAI


Requirements
------------


Backwards Compatibility
=======================

Backwards compatibility to current systems is ensured by supporting established frameworks and only providing light weight abstractions and supporting components around it.


Security Impact
===============


Safety Impact
=============


License Impact
==============


How to Teach This
=================


Rejected Ideas
==============


Open Issues
===========

- GPU shared memory data pipeline
- ONNX support on QNX


Footnotes
=========

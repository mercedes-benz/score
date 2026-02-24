#ifndef SCORE_INTERFACES_HPP
#define SCORE_INTERFACES_HPP

#include <cstdint>

// Base Score types
#include <array>
#include <chrono>
#include <vector>
#include <string>
#include <string_view>
#include <type_traits>

// Score Logger
#include <score/log/Logger.hpp>

// Score-Fabric framework headers
#include <score/comm/Sample.hpp>
#include <score/comm/ShmSampleBuffer.hpp>
#include <score/middleware/TaskPortData.hpp>
#include <queue.hpp>

#include <adas/perception/PerformanceTrace.hpp>
#include <adas/perception/DetectedObject.hpp>
#include <adas/perception/LogEntry.hpp>
#include <adas/perception/ObjectDetections.hpp>
#include <adas/perception/BrakeCommand.hpp>
#include <adas/perception/TrafficSignCommand.hpp>
#include <adas/perception/CameraFrame.hpp>
#include <adas/perception/BoundingBox.hpp>
#include <adas/perception/DetectedTrafficSign.hpp>
#include <adas/perception/LogLevel.hpp>
#include <adas/perception/TrafficSignType.hpp>
#include <adas/perception/ObjectClass.hpp>
#include <adas/perception/BrakeSource.hpp>
#include <adas/perception/constexprs.hpp>

// Archetype parameters includes

// Bridge parameters includes

struct TrafficSignRecognitionInputs {
    std::queue<adas::perception::ObjectDetections, 1> object_detections_input_queue;
};

struct TrafficSignRecognitionOutputs {
    static constexpr std::size_t traffic_sign_commands_num_slots{ 2 };
    static constexpr std::size_t traffic_sign_commands_n_samples_max{ 1 };
    std::queue<score::comm::ShmMutableSampleBufferBase<adas::perception::TrafficSignCommand, traffic_sign_commands_num_slots>, traffic_sign_commands_n_samples_max> traffic_sign_commands_output_queue;
};

struct TrafficSignRecognitionParameters {
    float confidence_threshold { 0.85 };
    float max_detection_distance_m { 100 };
    bool enable_speed_limit_detection { true };
    bool enable_warning_signs { true };
    bool enable_regulatory_signs { true };
    bool sign_tracking_enabled { true };
    std::string model_path { "/models/traffic_signs_eu.onnx" };
    uint8_t max_signs_per_frame { 8 };
    uint16_t min_sign_size_pixels { 32 };
    uint8_t temporal_smoothing_frames { 5 };
};

struct TrafficSignRecognitionInternalState {
    uint64_t sign_processing_counter { 0 };
    std::chrono::time_point last_detection_timestamp {  };
    uint8_t current_speed_limit_kmh { 50 };
    uint8_t active_sign_count { 0 };
    bool model_lscoreed { false };
    uint64_t total_inference_time_us { 0 };
    uint32_t last_inference_time_us { 0 };
    uint32_t missed_frame_count { 0 };
    std::array<uint32_t, 40> sign_tracking_buffer {  };
    std::array<float, 40> confidence_history {  };
};

struct CameraControllerInputs {
};

struct CameraControllerOutputs {
    static constexpr std::size_t camera_frames_num_slots{ 3 };
    static constexpr std::size_t camera_frames_n_samples_max{ 1 };
    std::queue<score::comm::ShmMutableSampleBufferBase<adas::perception::CameraFrame, camera_frames_num_slots>, camera_frames_n_samples_max> camera_frames_output_queue;
};

struct CameraControllerParameters {
    uint8_t camera_id { 0 };
    uint32_t frame_rate_fps { 30 };
    uint32_t resolution_width { 1920 };
    uint32_t resolution_height { 1080 };
    bool exposure_auto { true };
    uint32_t exposure_time_us { 33333 };
    float gain { 1 };
};

struct CameraControllerInternalState {
    uint64_t frame_counter { 0 };
    std::chrono::time_point last_frame_timestamp {  };
    uint32_t dropped_frames { 0 };
    uint32_t current_exposure_us { 33333 };
    bool is_streaming { false };
};

struct NeuralNetInferenceInputs {
    std::queue<adas::perception::CameraFrame, 2> input_frames_input_queue;
};

struct NeuralNetInferenceOutputs {
    static constexpr std::size_t detections_num_slots{ 4 };
    static constexpr std::size_t detections_n_samples_max{ 1 };
    std::queue<score::comm::ShmMutableSampleBufferBase<adas::perception::ObjectDetections, detections_num_slots>, detections_n_samples_max> detections_output_queue;
};

struct NeuralNetInferenceParameters {
    std::string model_path { "/models/default.onnx" };
    float confidence_threshold { 0.7 };
    float nms_threshold { 0.4 };
    uint16_t max_detections { 32 };
    uint8_t batch_size { 1 };
    int32_t gpu_device_id { 0 };
};

struct NeuralNetInferenceInternalState {
    bool model_lscoreed { false };
    uint64_t inference_counter { 0 };
    uint64_t total_inference_time_us { 0 };
    uint32_t last_inference_time_us { 0 };
    uint32_t gpu_memory_allocated_mb { 0 };
    std::array<float, 6220800> preprocessing_buffer {  };
};

struct SystemLoggerInputs {
    std::queue<adas::perception::CameraFrame, 5> camera_data_log_input_queue;
    std::queue<adas::perception::ObjectDetections, 5> detection_log_input_queue;
    std::queue<adas::perception::BrakeCommand, 10> brake_command_log_input_queue;
    std::queue<adas::perception::TrafficSignCommand, 5> traffic_sign_log_input_queue;
};

struct SystemLoggerOutputs {
    static constexpr std::size_t log_entries_num_slots{ 2 };
    static constexpr std::size_t log_entries_n_samples_max{ 10 };
    std::queue<score::comm::ShmMutableSampleBufferBase<adas::perception::LogEntry, log_entries_num_slots>, log_entries_n_samples_max> log_entries_output_queue;
};

struct SystemLoggerParameters {
    uint8_t log_level { 1 };
    std::string log_file_path { "/var/log/adas.log" };
    uint32_t max_file_size_mb { 100 };
    bool enable_camera_logging { true };
    bool enable_detection_logging { true };
    bool enable_brake_logging { true };
    bool enable_traffic_sign_logging { true };
};

struct SystemLoggerInternalState {
    uint64_t log_entries_written { 0 };
    uint32_t current_log_file_size_mb { 0 };
    uint16_t log_buffer_count { 0 };
    uint32_t dropped_log_entries { 0 };
    std::chrono::time_point last_flush_timestamp {  };
};

struct BrakeControllerInputs {
    std::queue<adas::perception::ObjectDetections, 1> object_detections_input_queue;
};

struct BrakeControllerOutputs {
    static constexpr std::size_t brake_commands_num_slots{ 2 };
    static constexpr std::size_t brake_commands_n_samples_max{ 1 };
    std::queue<score::comm::ShmMutableSampleBufferBase<adas::perception::BrakeCommand, brake_commands_num_slots>, brake_commands_n_samples_max> brake_commands_output_queue;
};

struct BrakeControllerParameters {
    float ttc_threshold_seconds { 3 };
    float max_brake_pressure_bar { 200 };
    float emergency_deceleration_mps2 { 9 };
    bool enable_emergency_braking { true };
    float min_object_distance_m { 2 };
};

struct BrakeControllerInternalState {
    std::chrono::time_point last_brake_command_time {  };
    bool emergency_brake_active { false };
    float current_brake_pressure_bar { 0 };
    uint64_t brake_command_counter { 0 };
    float closest_object_distance_m { 999 };
    float last_ttc_calculation { 999 };
};

struct PerformanceTracerInputs {
    std::queue<adas::perception::CameraFrame, 3> camera_perf_trace_input_queue;
    std::queue<adas::perception::ObjectDetections, 3> neural_net_perf_trace_input_queue;
    std::queue<adas::perception::BrakeCommand, 10> brake_perf_trace_input_queue;
    std::queue<adas::perception::TrafficSignCommand, 3> traffic_sign_perf_trace_input_queue;
};

struct PerformanceTracerOutputs {
    static constexpr std::size_t performance_traces_num_slots{ 2 };
    static constexpr std::size_t performance_traces_n_samples_max{ 5 };
    std::queue<score::comm::ShmMutableSampleBufferBase<adas::perception::PerformanceTrace, performance_traces_num_slots>, performance_traces_n_samples_max> performance_traces_output_queue;
};

struct PerformanceTracerParameters {
    uint32_t sampling_rate_hz { 100 };
    std::string trace_file_path { "/var/log/performance.trace" };
    bool cpu_monitoring_enabled { true };
    bool memory_monitoring_enabled { true };
    bool timing_analysis_enabled { true };
    uint32_t buffer_size_entries { 1000 };
};

struct PerformanceTracerInternalState {
    uint64_t trace_entries_collected { 0 };
    uint32_t current_trace_buffer_size { 0 };
    uint32_t performance_samples_count { 0 };
    float last_cpu_usage_percent { 0 };
    uint32_t last_memory_usage_mb { 0 };
    bool trace_buffer_full { false };
};

 
struct Front_cameraInputReferences {
};

struct Object_detectorInputReferences {
  std::vector<score::comm::SampleReference> input_frames_sample_refs(2);
};

struct Traffic_sign_controllerInputReferences {
  std::vector<score::comm::SampleReference> object_detections_sample_refs(1);
};

struct Emergency_brake_ctrlInputReferences {
  std::vector<score::comm::SampleReference> object_detections_sample_refs(1);
};

struct System_data_loggerInputReferences {
  std::vector<score::comm::SampleReference> camera_data_log_sample_refs(5);
  std::vector<score::comm::SampleReference> detection_log_sample_refs(5);
  std::vector<score::comm::SampleReference> brake_command_log_sample_refs(10);
  std::vector<score::comm::SampleReference> traffic_sign_log_sample_refs(5);
};

struct System_perf_monitorInputReferences {
  std::vector<score::comm::SampleReference> camera_perf_trace_sample_refs(3);
  std::vector<score::comm::SampleReference> neural_net_perf_trace_sample_refs(3);
  std::vector<score::comm::SampleReference> brake_perf_trace_sample_refs(10);
  std::vector<score::comm::SampleReference> traffic_sign_perf_trace_sample_refs(3);
};

 
template <typename TEnum, typename std::enable_if<std::is_enum<TEnum>::value, TEnum>::type* = nullptr>
[[nodiscard]] inline std::expected<TEnum> fromString(const std::string_view) noexcept {
    return std::error{std::errorCode::kNotSupported};
}

template<>
inline std::expected<adas::perception::LogLevel> fromString(const std::string_view str) noexcept {
    if (str == "adas::perception::LogLevel::kDebug") {
        return { adas::perception::LogLevel::kDebug };
    }
    if (str == "adas::perception::LogLevel::kInfo") {
        return { adas::perception::LogLevel::kInfo };
    }
    if (str == "adas::perception::LogLevel::kWarning") {
        return { adas::perception::LogLevel::kWarning };
    }
    if (str == "adas::perception::LogLevel::kError") {
        return { adas::perception::LogLevel::kError };
    }
    if (str == "adas::perception::LogLevel::kFatal") {
        return { adas::perception::LogLevel::kFatal };
    }
    return std::error{std::errorCode::kInvalidArgument};
}
template<>
inline std::expected<adas::perception::TrafficSignType> fromString(const std::string_view str) noexcept {
    if (str == "adas::perception::TrafficSignType::kUnknownSign") {
        return { adas::perception::TrafficSignType::kUnknownSign };
    }
    if (str == "adas::perception::TrafficSignType::kSpeedLimit30") {
        return { adas::perception::TrafficSignType::kSpeedLimit30 };
    }
    if (str == "adas::perception::TrafficSignType::kSpeedLimit50") {
        return { adas::perception::TrafficSignType::kSpeedLimit50 };
    }
    if (str == "adas::perception::TrafficSignType::kSpeedLimit70") {
        return { adas::perception::TrafficSignType::kSpeedLimit70 };
    }
    if (str == "adas::perception::TrafficSignType::kSpeedLimit90") {
        return { adas::perception::TrafficSignType::kSpeedLimit90 };
    }
    if (str == "adas::perception::TrafficSignType::kSpeedLimit130") {
        return { adas::perception::TrafficSignType::kSpeedLimit130 };
    }
    if (str == "adas::perception::TrafficSignType::kSpeedLimitEnd") {
        return { adas::perception::TrafficSignType::kSpeedLimitEnd };
    }
    if (str == "adas::perception::TrafficSignType::kStop") {
        return { adas::perception::TrafficSignType::kStop };
    }
    if (str == "adas::perception::TrafficSignType::kYield") {
        return { adas::perception::TrafficSignType::kYield };
    }
    if (str == "adas::perception::TrafficSignType::kNoEntry") {
        return { adas::perception::TrafficSignType::kNoEntry };
    }
    if (str == "adas::perception::TrafficSignType::kConstruction") {
        return { adas::perception::TrafficSignType::kConstruction };
    }
    if (str == "adas::perception::TrafficSignType::kSchoolZone") {
        return { adas::perception::TrafficSignType::kSchoolZone };
    }
    return std::error{std::errorCode::kInvalidArgument};
}
template<>
inline std::expected<adas::perception::ObjectClass> fromString(const std::string_view str) noexcept {
    if (str == "adas::perception::ObjectClass::kUnknown") {
        return { adas::perception::ObjectClass::kUnknown };
    }
    if (str == "adas::perception::ObjectClass::kVehicle") {
        return { adas::perception::ObjectClass::kVehicle };
    }
    if (str == "adas::perception::ObjectClass::kPedestrian") {
        return { adas::perception::ObjectClass::kPedestrian };
    }
    if (str == "adas::perception::ObjectClass::kCyclist") {
        return { adas::perception::ObjectClass::kCyclist };
    }
    if (str == "adas::perception::ObjectClass::kTrafficSign") {
        return { adas::perception::ObjectClass::kTrafficSign };
    }
    if (str == "adas::perception::ObjectClass::kTrafficLight") {
        return { adas::perception::ObjectClass::kTrafficLight };
    }
    if (str == "adas::perception::ObjectClass::kObstacle") {
        return { adas::perception::ObjectClass::kObstacle };
    }
    return std::error{std::errorCode::kInvalidArgument};
}
template<>
inline std::expected<adas::perception::BrakeSource> fromString(const std::string_view str) noexcept {
    if (str == "adas::perception::BrakeSource::kManual") {
        return { adas::perception::BrakeSource::kManual };
    }
    if (str == "adas::perception::BrakeSource::kAEB") {
        return { adas::perception::BrakeSource::kAEB };
    }
    if (str == "adas::perception::BrakeSource::kACC") {
        return { adas::perception::BrakeSource::kACC };
    }
    if (str == "adas::perception::BrakeSource::kCollisionAvoidance") {
        return { adas::perception::BrakeSource::kCollisionAvoidance };
    }
    return std::error{std::errorCode::kInvalidArgument};
}

#endif  // SCORE_INTERFACES_HPP

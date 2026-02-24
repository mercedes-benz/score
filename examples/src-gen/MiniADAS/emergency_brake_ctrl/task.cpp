#include <runnable.hpp>

#include <getopt.h>
#include <libgen.h>

#include <atomic>
#include <csignal>
#include <cstdint>
#include <type_traits>

#include <score/base/Console.hpp>
#include <system_error>
#include <cstdint>
#include <score/log/Logger.hpp>
#include <score/middleware/Port.hpp>
#include <score/middleware/TaskBase.hpp>
#include <score/middleware/Util.hpp>
#include <score/util/Defer.hpp>

static_assert(sizeof(BrakeController) == sizeof(score::middleware::RunnableLayout<BrakeControllerInputs,
                                                                                   BrakeControllerOutputs,
                                                                                   BrakeControllerInternalState,
                                                                                   BrakeControllerParameters,
                                                                                   BrakeControllerHiddenState>),
              "BrakeController shouldn't contain extra data members");

static constexpr std::string_view kLogPrefix{"emergency_brake_ctrl"};


// Task class definition for containing Runnable Instance 'emergency_brake_ctrl'
class Emergency_brake_ctrlTask final : public score::middleware::TaskBase<Emergency_brake_ctrlInputReferences, BrakeController> {
  public:
    using base_type = score::middleware::TaskBase<Emergency_brake_ctrlInputReferences, BrakeController>;
    using parameters_type = typename base_type::parameters_type;
    using input_references_type = typename base_type::input_references_type;
    using internal_state_type = typename base_type::internal_state_type;
    using task_output_data_type = typename base_type::task_output_data_type;

  protected:
    std::error initInputs() noexcept final {
        std::error result{std::errorCode::kSuccess};
        // Initialize the private members for input buffers
        // open MiniADAS:ObjectDetection.object_detector:detected_objects
        result |= initInput("MiniADAS",
                            "ObjectDetection",
                            "object_detector",
                            "detected_objects",
                            object_detections_buffer_);
        return result;
    }

    std::error initOutputs() noexcept final {
        std::error result{std::errorCode::kSuccess};
        // open MiniADAS:BrakeControl.emergency_brake_ctrl:brake_commands
        result |= initOutput(score::comm::SampleStorageType::kShmSampleBuffer,
                             "MiniADAS",
                             "BrakeControl",
                             "emergency_brake_ctrl",
                             "brake_commands",
                             brake_commands_buffer_);
        return result;
    }

    std::error process([[maybe_unused]] const std::chrono::time_point& time,
                             [[maybe_unused]] const parameters_type& parameters,
                             [[maybe_unused]] const input_references_type& inputs,
                             [[maybe_unused]] internal_state_type& internal_state,
                             [[maybe_unused]] task_output_data_type& outputs) noexcept final {
        // Handle the input references and populate an Inputs structure with them for use with onUpdate()
      std::vector<const adas::perception::ObjectDetections*> object_detections_received(1);
        for (const score::comm::SampleReference& sample_ref : inputs.object_detections_sample_refs) {
            this->object_detections_buffer_.getEntry(sample_ref.slot())
                .with_value([&object_detections_received](score::base::ReferenceWrapper<const score::comm::SampleBufferEntry<adas::perception::ObjectDetections, score::comm::kDefaultPageSize>>& sample_data) {
                    object_detections_received.push_back(&sample_data.get().sample.get());
                })
                .with_error([&sample_ref](const std::error& error) {
                    score::log::error(kLogPrefix, "Unable to find input sample MiniADAS:ObjectDetection.object_detector:detected_objects ", sample_ref.slot(), ": ", error);
                });
        }
        // Construct the Inputs structure for the runnable
        // NOTE: relying on order instead of using designated initializer (since this is a C++20 feature)
        BrakeControllerInputs input_buffers {
            std::queue{ object_detections_received }
        };

        // Containers for the output processing for-loops
        using brake_commands_vector_element_type =
            std::pair<score::comm::SampleReference::slot_type, score::comm::ShmMutableSampleBufferBase<adas::perception::BrakeCommand, 2>::mutable_sample_type>;
        std::vector<brake_commands_vector_element_type> brake_commands_vector(1);

        // Construct the output data structure for the runnable
        // NOTE: relying on order instead of using designated initializer (since this is a C++20 feature)
        BrakeControllerOutputs output_buffers{
            std::queue<score::comm::ShmMutableSampleBufferBase<adas::perception::BrakeCommand, 2>, 1> {
                brake_commands_buffer_,
                brake_commands_vector
            }
        };
        const std::error onupdate_result = runnable_.onUpdate(time, parameters, input_buffers, internal_state, output_buffers);
        if (onupdate_result.success()) {
            for (brake_commands_vector_element_type& pair : brake_commands_vector) {
                pair.second.publish(time, brake_commands_sequence_counter_);
                outputs.push(score::comm::SampleReference{ kBrakeCommandsTopicId, pair.first, time, brake_commands_sequence_counter_});
                brake_commands_sequence_counter_++;
            }
        }
        return onupdate_result;
    }

    std::error release([[maybe_unused]] typename score::middleware::TaskData<Emergency_brake_ctrlInputReferences>::sample_release_queue& release_samples) noexcept final {
        while (release_samples.size() > 0) {
            release_samples.front().with_value([&](const score::base::ReferenceWrapper<score::comm::SampleReference>& sample_ref) {
                if (sample_ref.get().topicId() == kBrakeCommandsTopicId) {
                    score::log::debug3(kLogPrefix, "Sample buffer is releasing sample with TopicID ",
                                     score::comm::SampleReferenceFormatter{sample_ref});
                    brake_commands_buffer_.release(sample_ref.get().slot());
                }
                else
                {
                    score::log::error(kLogPrefix, "Unable to release unknown sample ID ", score::comm::SampleReferenceFormatter{sample_ref});
                }
            });
            release_samples.pop();
        }
        return std::error{std::errorCode::kSuccess};
    }

  private:
    // We need an SampleBuffer for each Archetype Input. The number of slots needs to match that of the producer runnable's output buffer.
    static constexpr score::comm::TopicId kObjectDetectionsTopicId{ score::comm::SampleStorageType::kShmSampleBuffer, 0xd75d18345a52fdf9, 0 };
    score::comm::ShmSampleBuffer<adas::perception::ObjectDetections, 4> object_detections_buffer_{0};

    // We need a Counter and Buffer for each member in struct Outputs
    score::comm::SampleReference::sequence_number_type brake_commands_sequence_counter_{0};
    static constexpr score::comm::TopicId kBrakeCommandsTopicId{ score::comm::SampleStorageType::kShmSampleBuffer, 0x502077a899668e1c, 0 };
    score::comm::ShmMutableSampleBuffer<adas::perception::BrakeCommand, 2> brake_commands_buffer_{0};
};

/* This is an intentional deviation of MISRA C++:2023 Rule 6.7.2 Global variables shall not be used. The object is used
 * in a signal handler whose registration is already a global state equivalent to a global variable, and this ensures
 * that the object is valid and accessible whenever the signal handler runs during program execution. */
// coverity[misra_cpp_2023_rule_6_7_2_violation]
static Emergency_brake_ctrlTask task{};

// Boilerplate follows

static constexpr int kOptArgNoArgument{0};
static constexpr int kOptArgRequiredArgument{1};
static constexpr int kOptArgNull{0};

static void signal_handler([[maybe_unused]] int signum) {
    task.shutdown();
}

static void usage(std::string_view argv0) {
    const auto pos = argv0.find_last_of("/");
    score::base::Console::out("Usage: ", argv0.data() + (((pos == argv0.npos) || (pos == (argv0.size() - 1))) ? 0 : (pos + 1)),
                            " [-l<x>|--loglevel=<x>] [-h|--help]");
    score::base::Console::out("-l|--loglevel=<x>:  Minimum log level (0...2: debug, 3: info, 4: warn, 5: error, 6: fatal) (default: 3)");
    score::base::Console::out("-h|--help:          Print this message.");
    score::base::Console::flush();
}

/* This is a false-positive Coverity detection of MISRA C++:2023 Rule 6.2.1 The one-definition rule shall not be
 * violated. Every Score instance's main function in task.cpp is detected as a one-definition rule violation even
 * though the instances are independent Bazel binary targets. Instance Bazel binary targets are not included as part of
 * other binary targets's build dependencies. If that was the case, the code compilation would fail because multiple
 * main functions are not allowed in one binary target. */
// coverity[misra_cpp_2023_rule_6_2_1_violation:FALSE]
int main(int argc, char* argv[]) {
    /* This deviation from MISRA C++:2023 rule 21.10.3 "The facilities provided by <csignal> shall not be used"
     * is intentional here. We have made sure that our signal handler functions are signal safe: They either
     * a) don't do anything (the signal is caught to interrupt an mq_timedreceive call elsewhere) or
     * b) at most, they atomically set a global flag variable that is handled later in the main program.
     */
    for( const auto& sig_value : {SIGINT, SIGTERM}) {
        // coverity[misra_cpp_2023_rule_21_10_3_violation]
        if (std::signal(sig_value, signal_handler) == SIG_ERR) {
            score::base::Console::out(kLogPrefix, ": Failed to install ", ((sig_value == SIGINT) ? "SIGINT" : "SIGTERM"), " handler");
            return EXIT_FAILURE;
        }
    }

    score::log::LogLevel log_level{score::log::kDefaultLogLevel};

    score::log::Logger::registerStandardSink(log_level, std::string_view{"Emergency_brake_ctrlTASK"});
    const auto on_scope_exit = score::util::makeDefer([]() noexcept {
        // flush log data before closing node
        score::log::Logger::flush();
    });

    const struct option long_options[] {
        {"help",        kOptArgNoArgument,        nullptr,     'h'},
        {"loglevel",    kOptArgRequiredArgument,  nullptr,     'l'},
        {nullptr,       kOptArgNull,              nullptr,     kOptArgNull}
    };
    int c;
    while ((c = ::getopt_long(argc, argv, "hl:", long_options, nullptr)) != -1) {
        switch (c) {
            case 'h':
                usage(argv[0]);
                return EXIT_SUCCESS;
            case 'l': {
                const auto log_level_value{static_cast<score::base::int32_t>(optarg[0] - '0')};
                // optarg must be a single-character string that converts to a number in the log level range
                if ((optarg[1] != '\0') ||
                    (log_level_value < score::base::to_underlying(score::log::LogLevel::kDebug3)) ||
                    (log_level_value > score::base::to_underlying(score::log::LogLevel::kFatal))) {
                    score::log::error(kLogPrefix, "Invalid log level argument '",
                                    optarg, "'; must be a number between ",
                                    score::base::to_underlying(score::log::LogLevel::kDebug3), " and ",
                                    score::base::to_underlying(score::log::LogLevel::kFatal));
                    return EXIT_FAILURE;
                }
                log_level = static_cast<score::log::LogLevel>(log_level_value);
                score::log::Logger::setMinimumLogLevel(log_level);
                }
                break;
            default:
                score::log::error(kLogPrefix, "Unknown option '", static_cast<char>(optopt), "'.");
                usage(argv[0]);
                return EXIT_FAILURE;
        }
    }

    // initialize task
    std::error score_result{task.init(score::middleware::PortName{"MiniADAS", "BrakeController", "emergency_brake_ctrl"})};
    int process_result{EXIT_SUCCESS};
    if (score_result.failure()) {
        score::log::error(kLogPrefix, "Failed to initialize task (", score_result, ")");
        process_result = EXIT_FAILURE;
    }
    // run task
    else if ((score_result = task.run()).failure()) {
        score::log::error(kLogPrefix, "Running task failed (", score_result, ")");
        process_result = EXIT_FAILURE;
    } else {
    }

    return process_result;
}

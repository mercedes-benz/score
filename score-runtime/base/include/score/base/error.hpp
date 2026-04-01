#pragma once

/// @file error.hpp
/// @brief Error codes and result type for the score runtime.

namespace score::base {

enum class ErrorCodeValue {
    kSuccess = 0,
    kFailed = 1,
};

class ErrorCode {
  public:
    ErrorCode() : value_{ErrorCodeValue::kFailed} {}
    explicit ErrorCode(ErrorCodeValue value) : value_{value} {}

    bool IsSuccess() const { return value_ == ErrorCodeValue::kSuccess; }
    bool IsFailed() const { return value_ == ErrorCodeValue::kFailed; }

    ErrorCodeValue Value() const { return value_; }

    bool operator==(const ErrorCode& other) const { return value_ == other.value_; }
    bool operator!=(const ErrorCode& other) const { return value_ != other.value_; }

    static ErrorCode Success() { return ErrorCode{ErrorCodeValue::kSuccess}; }
    static ErrorCode Failed() { return ErrorCode{ErrorCodeValue::kFailed}; }

  private:
    ErrorCodeValue value_;
};

} // namespace score::base

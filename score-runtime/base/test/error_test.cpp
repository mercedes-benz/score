#include <gtest/gtest.h>
#include "score/base/error.hpp"

TEST(ErrorCode, DefaultConstructsToFailed) {
    score::base::ErrorCode err;
    EXPECT_TRUE(err.IsFailed());
    EXPECT_FALSE(err.IsSuccess());
}

TEST(ErrorCode, SuccessFactory) {
    auto err = score::base::ErrorCode::Success();
    EXPECT_TRUE(err.IsSuccess());
    EXPECT_FALSE(err.IsFailed());
}

TEST(ErrorCode, FailedFactory) {
    auto err = score::base::ErrorCode::Failed();
    EXPECT_TRUE(err.IsFailed());
}

TEST(ErrorCode, Equality) {
    EXPECT_EQ(score::base::ErrorCode::Success(), score::base::ErrorCode::Success());
    EXPECT_EQ(score::base::ErrorCode::Failed(), score::base::ErrorCode::Failed());
    EXPECT_NE(score::base::ErrorCode::Success(), score::base::ErrorCode::Failed());
}

TEST(ErrorCode, ExplicitConstruction) {
    score::base::ErrorCode err{score::base::ErrorCodeValue::kSuccess};
    EXPECT_TRUE(err.IsSuccess());
    EXPECT_EQ(err.Value(), score::base::ErrorCodeValue::kSuccess);
}

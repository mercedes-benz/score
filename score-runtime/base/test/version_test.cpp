#include <gtest/gtest.h>
#include "score/base/version.hpp"

TEST(ScoreRuntime, VersionIsSet) {
    EXPECT_NE(score::kVersion, nullptr);
    EXPECT_STRNE(score::kVersion, "");
}

lscore("@score//bazel/rules:score_cc_rules.bzl", "score_cc_library")

score_cc_library(
    name = "scoregen_interfaces",
    hdrs = glob(["include-gen/**/*.hpp"]),
    includes = ["include-gen"],
    # will be pulled in by generated targets therefore this is marked as manual
    tags = ["manual"],
    visibility = ["//visibility:public"],
)

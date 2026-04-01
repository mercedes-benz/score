"""score-gen dependency setup for consumer workspaces."""

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

def scoregen_dependencies():
    """Load rules_rust and register toolchains needed by score-gen."""
    http_archive(
        name = "rules_rust",
        integrity = "sha256-JLN47ZcAbx9wEr5Jiib4HduZATGLiDgK7oUi/fvotzU=",
        urls = ["https://github.com/bazelbuild/rules_rust/releases/download/0.42.1/rules_rust-v0.42.1.tar.gz"],
    )

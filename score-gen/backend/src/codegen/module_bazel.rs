use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;

use super::render;

const DEFAULT_MODULE_BAZEL_TEMPLATE: &str = include_str!("../../templates/MODULE.bazel.j2");

/// Template context for generating the top-level MODULE.bazel.
#[derive(Serialize)]
struct ModuleBazelContext {
    module_name: String,
    score_runtime_path: String,
}

/// Generate a MODULE.bazel file at the project root.
///
/// `project_root` is the directory where MODULE.bazel will be written.
/// `module_name` is the Bazel module name (typically the system name).
/// `score_runtime_path` is the relative path from project_root to score-runtime.
pub fn generate_module_bazel(
    project_root: &Path,
    module_name: &str,
    score_runtime_path: &str,
    template_dir: Option<&Path>,
) -> Result<()> {
    let template_src = if let Some(dir) = template_dir {
        let p = dir.join("MODULE.bazel.j2");
        fs::read_to_string(&p)
            .with_context(|| format!("Failed to read template {}", p.display()))?
    } else {
        DEFAULT_MODULE_BAZEL_TEMPLATE.to_string()
    };
    let ctx = ModuleBazelContext {
        module_name: module_name.to_string(),
        score_runtime_path: score_runtime_path.to_string(),
    };
    let content = render(&template_src, &ctx)
        .context("Failed to render MODULE.bazel")?;
    let out_path = project_root.join("MODULE.bazel");
    fs::write(&out_path, &content)
        .with_context(|| format!("Failed to write {}", out_path.display()))?;
    Ok(())
}

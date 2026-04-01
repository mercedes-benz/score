mod generate;
mod generate_tasks;
mod interfaces;
mod internal_state;
mod module_bazel;
mod parameters;
mod runnable;
mod task;

pub use generate::generate;
pub use generate_tasks::generate_tasks;
pub use interfaces::generate_interfaces;
pub use module_bazel::generate_module_bazel;

use anyhow::Result;
use minijinja::Environment;
use serde::Serialize;

pub(crate) const DEFAULT_PARAMS_TEMPLATE: &str = include_str!("../../templates/parameters.hpp.j2");
pub(crate) const DEFAULT_STATE_TEMPLATE: &str = include_str!("../../templates/internal_state.hpp.j2");

/// Convert a PascalCase or camelCase name to snake_case.
/// Consecutive uppercase letters are treated as a single acronym word.
pub fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = name.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            let prev_upper = i > 0 && chars[i - 1].is_uppercase();
            let next_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();
            if i > 0 && (!prev_upper || next_lower) {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

/// A single member in a generated struct, for template rendering.
#[derive(Serialize)]
pub(crate) struct MemberContext {
    pub name: String,
    pub member_type: String,
    pub default_value: Option<String>,
}

/// Format a serde_yaml::Value as a C++ literal.
pub(crate) fn format_cpp_default(val: &serde_yaml::Value, member_type: &str) -> String {
    match val {
        serde_yaml::Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        serde_yaml::Value::Number(n) => {
            let s = n.to_string();
            if member_type == "float" && !s.contains('.') {
                format!("{}.0f", s)
            } else if member_type == "float" {
                format!("{}f", s)
            } else if member_type == "double" && !s.contains('.') {
                format!("{}.0", s)
            } else {
                s
            }
        }
        serde_yaml::Value::String(s) => format!("\"{}\"", s),
        _ => format!("{}", val.as_str().unwrap_or("")),
    }
}

/// Map a YAML type name to the corresponding C++ type.
pub(crate) fn to_cpp_type(yaml_type: &str) -> String {
    // Handle array types: "type[N]" → "std::array<cpp_type, N>"
    if let Some(bracket_pos) = yaml_type.find('[') {
        let base = &yaml_type[..bracket_pos];
        let size = &yaml_type[bracket_pos + 1..yaml_type.len() - 1];
        let cpp_base = to_cpp_type(base);
        return format!("std::array<{}, {}>", cpp_base, size);
    }
    match yaml_type {
        "string" => "std::string".to_string(),
        "time_point" => "std::chrono::time_point<std::chrono::steady_clock>".to_string(),
        other => other.to_string(),
    }
}

/// Render a template string with the given serializable context.
pub(crate) fn render<T: Serialize>(template_src: &str, ctx: &T) -> Result<String> {
    let mut env = Environment::new();
    env.add_template("tpl", template_src)?;
    let tmpl = env.get_template("tpl")?;
    Ok(tmpl.render(ctx)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("CameraController"), "camera_controller");
        assert_eq!(to_snake_case("NeuralNetInference"), "neural_net_inference");
        assert_eq!(to_snake_case("SystemLogger"), "system_logger");
        assert_eq!(to_snake_case("BrakeController"), "brake_controller");
        assert_eq!(to_snake_case("GPU"), "gpu");
    }

    #[test]
    fn test_to_snake_case_already_snake() {
        assert_eq!(to_snake_case("already_snake"), "already_snake");
    }

    #[test]
    fn test_to_snake_case_acronyms() {
        assert_eq!(to_snake_case("ADAS"), "adas");
        assert_eq!(to_snake_case("HTMLParser"), "html_parser");
        assert_eq!(to_snake_case("parseJSON"), "parse_json");
    }

    #[test]
    fn test_format_cpp_default_values() {
        // Bool
        assert_eq!(
            format_cpp_default(&serde_yaml::Value::Bool(true), "bool"),
            "true"
        );
        assert_eq!(
            format_cpp_default(&serde_yaml::Value::Bool(false), "bool"),
            "false"
        );
        // Integer
        let v: serde_yaml::Value = serde_yaml::from_str("42").unwrap();
        assert_eq!(format_cpp_default(&v, "uint32_t"), "42");
        // Float with decimal
        let v: serde_yaml::Value = serde_yaml::from_str("1.5").unwrap();
        assert_eq!(format_cpp_default(&v, "float"), "1.5f");
        // Float without decimal
        let v: serde_yaml::Value = serde_yaml::from_str("30").unwrap();
        assert_eq!(format_cpp_default(&v, "float"), "30.0f");
        // Double without decimal
        let v: serde_yaml::Value = serde_yaml::from_str("30").unwrap();
        assert_eq!(format_cpp_default(&v, "double"), "30.0");
        // String
        let v: serde_yaml::Value = serde_yaml::from_str("\"hello\"").unwrap();
        assert_eq!(format_cpp_default(&v, "string"), "\"hello\"");
    }

    #[test]
    fn test_to_cpp_type_array() {
        assert_eq!(to_cpp_type("float[6220800]"), "std::array<float, 6220800>");
        assert_eq!(to_cpp_type("uint32_t[40]"), "std::array<uint32_t, 40>");
        assert_eq!(to_cpp_type("string[10]"), "std::array<std::string, 10>");
        assert_eq!(to_cpp_type("uint8_t"), "uint8_t");
        assert_eq!(to_cpp_type("string"), "std::string");
    }
}

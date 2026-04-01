use anyhow::{Context, Result};
use frontend::interfaces::InterfaceList;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::{format_cpp_default, render, to_cpp_type, to_snake_case};

const DEFAULT_INTERFACE_STRUCT_TEMPLATE: &str = include_str!("../../templates/interface_struct.hpp.j2");
const DEFAULT_INTERFACE_ENUM_TEMPLATE: &str = include_str!("../../templates/interface_enum.hpp.j2");
const DEFAULT_INTERFACE_CONSTEXPR_TEMPLATE: &str = include_str!("../../templates/interface_constexpr.hpp.j2");

/// Loaded template sources for interface code generation.
struct InterfaceTemplates {
    interface_struct: String,
    interface_enum: String,
    interface_constexpr: String,
}

/// Load interface templates from a directory if provided, or use built-in defaults.
fn load_interface_templates(template_dir: Option<&Path>) -> Result<InterfaceTemplates> {
    if let Some(dir) = template_dir {
        let struct_path = dir.join("interface_struct.hpp.j2");
        let enum_path = dir.join("interface_enum.hpp.j2");
        let constexpr_path = dir.join("interface_constexpr.hpp.j2");

        Ok(InterfaceTemplates {
            interface_struct: fs::read_to_string(&struct_path)
                .with_context(|| format!("Failed to read template {}", struct_path.display()))?,
            interface_enum: fs::read_to_string(&enum_path)
                .with_context(|| format!("Failed to read template {}", enum_path.display()))?,
            interface_constexpr: fs::read_to_string(&constexpr_path)
                .with_context(|| format!("Failed to read template {}", constexpr_path.display()))?,
        })
    } else {
        Ok(InterfaceTemplates {
            interface_struct: DEFAULT_INTERFACE_STRUCT_TEMPLATE.to_string(),
            interface_enum: DEFAULT_INTERFACE_ENUM_TEMPLATE.to_string(),
            interface_constexpr: DEFAULT_INTERFACE_CONSTEXPR_TEMPLATE.to_string(),
        })
    }
}

/// Template context for rendering an interface struct header.
#[derive(Serialize)]
pub(crate) struct InterfaceStructContext {
    pub guard: String,
    pub namespace: String,
    pub struct_name: String,
    pub members: Vec<InterfaceMemberContext>,
    pub system_includes: Vec<String>,
    pub local_includes: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct InterfaceMemberContext {
    pub name: String,
    pub member_type: String,
    pub default_value: Option<String>,
}

/// Template context for rendering an interface enum header.
#[derive(Serialize)]
pub(crate) struct InterfaceEnumContext {
    pub guard: String,
    pub namespace: String,
    pub enum_name: String,
    pub base_type: String,
    pub values: Vec<EnumValueContext>,
}

#[derive(Serialize)]
pub(crate) struct EnumValueContext {
    pub name: String,
    pub value: Option<i64>,
}

/// Template context for rendering an interface constexpr header.
#[derive(Serialize)]
pub(crate) struct InterfaceConstexprContext {
    pub guard: String,
    pub namespace: String,
    pub name: String,
    pub member_type: String,
    pub value: String,
    pub needs_cstdint: bool,
}

// --- Helpers ---

const PRIMITIVE_TYPES: &[&str] = &[
    "uint8_t", "uint16_t", "uint32_t", "uint64_t",
    "int8_t", "int16_t", "int32_t", "int64_t",
    "float", "double", "bool", "string", "size_t",
    "char", "void",
];

fn is_primitive(base_type: &str) -> bool {
    PRIMITIVE_TYPES.contains(&base_type)
}

/// Strip array brackets: "uint8_t[1024]" → "uint8_t"
fn strip_array(type_str: &str) -> &str {
    match type_str.find('[') {
        Some(pos) => &type_str[..pos],
        None => type_str,
    }
}

/// Build include guard: "adas::perception", "CameraFrame" → "ADAS_PERCEPTION_CAMERA_FRAME_HPP"
fn make_guard(namespace: &str, name: &str) -> String {
    let ns_parts: String = namespace
        .split("::")
        .map(|p| to_snake_case(p).to_uppercase())
        .collect::<Vec<_>>()
        .join("_");
    let type_part = to_snake_case(name).to_uppercase();
    format!("{}_{}_HPP", ns_parts, type_part)
}

fn needs_cstdint_type(base_type: &str) -> bool {
    base_type.starts_with("uint") || base_type.starts_with("int") || base_type == "size_t"
}

/// Determine system includes and local includes for a struct's members.
fn compute_struct_includes(
    members: &[frontend::interfaces::StructMemberDefinition],
) -> (Vec<String>, Vec<String>) {
    let mut sys = BTreeSet::new();
    let mut local = BTreeSet::new();

    for m in members {
        let base = strip_array(&m.member_type);
        if m.member_type.contains('[') {
            sys.insert("<array>".to_string());
        }
        if base == "string" {
            sys.insert("<string>".to_string());
        } else if needs_cstdint_type(base) {
            sys.insert("<cstdint>".to_string());
        } else if !is_primitive(base) {
            local.insert(format!("\"{}.hpp\"", to_snake_case(base)));
        }
    }

    (sys.into_iter().collect(), local.into_iter().collect())
}

// --- Build context from frontend types ---

impl InterfaceStructContext {
    pub fn from(namespace: &str, def: &frontend::interfaces::InterfaceStructDefinition) -> Self {
        let (system_includes, local_includes) = compute_struct_includes(&def.members);
        let members = def
            .members
            .iter()
            .map(|m| {
                let cpp_type = to_cpp_type(&m.member_type);
                let default_value = m
                    .default
                    .as_ref()
                    .map(|v| format_cpp_default(v, strip_array(&m.member_type)));
                InterfaceMemberContext {
                    name: m.name.clone(),
                    member_type: cpp_type,
                    default_value,
                }
            })
            .collect();

        InterfaceStructContext {
            guard: make_guard(namespace, &def.name),
            namespace: namespace.to_string(),
            struct_name: def.name.clone(),
            members,
            system_includes,
            local_includes,
        }
    }
}

impl InterfaceEnumContext {
    pub fn from(namespace: &str, def: &frontend::interfaces::EnumDefinition) -> Self {
        InterfaceEnumContext {
            guard: make_guard(namespace, &def.name),
            namespace: namespace.to_string(),
            enum_name: def.name.clone(),
            base_type: def.base.clone(),
            values: def
                .values
                .iter()
                .map(|v| EnumValueContext {
                    name: v.name.clone(),
                    value: v.value,
                })
                .collect(),
        }
    }
}

impl InterfaceConstexprContext {
    pub fn from(namespace: &str, def: &frontend::interfaces::ConstexprDefinition) -> Self {
        let cpp_type = to_cpp_type(&def.member_type);
        let needs_cstdint = needs_cstdint_type(&def.member_type);
        let value = format_cpp_default(&def.value, &def.member_type);
        InterfaceConstexprContext {
            guard: make_guard(namespace, &def.name),
            namespace: namespace.to_string(),
            name: def.name.clone(),
            member_type: cpp_type,
            value,
            needs_cstdint,
        }
    }
}

// --- Main generation function ---

/// Generate C++ interface header files into the output directory.
///
/// For each interface list, creates a subdirectory structure matching the namespace
/// (e.g., `adas::perception` → `adas/perception/`), then generates one `.hpp` file
/// per struct, enum, and constexpr definition.
pub fn generate_interfaces(
    output_dir: &Path,
    interface_lists: &[InterfaceList],
    template_dir: Option<&Path>,
) -> Result<Vec<String>> {
    let templates = load_interface_templates(template_dir)?;
    let mut generated = Vec::new();

    for list in interface_lists {
        // Convert namespace "adas::perception" → directory path "adas/perception"
        let ns_path: PathBuf = list.namespace.split("::").collect();
        let type_dir = output_dir.join(&ns_path);
        fs::create_dir_all(&type_dir)
            .with_context(|| format!("Failed to create directory {}", type_dir.display()))?;

        for s in &list.structs {
            let ctx = InterfaceStructContext::from(&list.namespace, s);
            let content = render(&templates.interface_struct, &ctx)
                .with_context(|| format!("Failed to render struct {}", s.name))?;
            let file_path = type_dir.join(format!("{}.hpp", to_snake_case(&s.name)));
            fs::write(&file_path, &content)
                .with_context(|| format!("Failed to write {}", file_path.display()))?;
            generated.push(format!("{}::{}", list.namespace, s.name));
        }

        for e in &list.enums {
            let ctx = InterfaceEnumContext::from(&list.namespace, e);
            let content = render(&templates.interface_enum, &ctx)
                .with_context(|| format!("Failed to render enum {}", e.name))?;
            let file_path = type_dir.join(format!("{}.hpp", to_snake_case(&e.name)));
            fs::write(&file_path, &content)
                .with_context(|| format!("Failed to write {}", file_path.display()))?;
            generated.push(format!("{}::{}", list.namespace, e.name));
        }

        for c in &list.constexprs {
            let ctx = InterfaceConstexprContext::from(&list.namespace, c);
            let content = render(&templates.interface_constexpr, &ctx)
                .with_context(|| format!("Failed to render constexpr {}", c.name))?;
            let file_path = type_dir.join(format!("{}.hpp", to_snake_case(&c.name)));
            fs::write(&file_path, &content)
                .with_context(|| format!("Failed to write {}", file_path.display()))?;
            generated.push(format!("{}::{}", list.namespace, c.name));
        }
    }

    // Write BUILD.bazel for the interfaces directory
    if !generated.is_empty() {
        let interfaces_build = "cc_library(\n    name = \"interfaces\",\n    hdrs = glob([\"**/*.hpp\"]),\n    includes = [\".\"],\n    visibility = [\"//visibility:public\"],\n)\n";
        let build_path = output_dir.join("BUILD.bazel");
        fs::write(&build_path, interfaces_build)
            .with_context(|| format!("Failed to write {}", build_path.display()))?;
    }

    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::render;
    use super::load_interface_templates;
    use frontend::interfaces::InterfaceList;

    #[test]
    fn test_make_guard() {
        assert_eq!(
            make_guard("adas::perception", "CameraFrame"),
            "ADAS_PERCEPTION_CAMERA_FRAME_HPP"
        );
        assert_eq!(make_guard("my::ns", "MyStruct"), "MY_NS_MY_STRUCT_HPP");
        assert_eq!(make_guard("test", "Color"), "TEST_COLOR_HPP");
    }

    #[test]
    fn test_render_struct_template() {
        let templates = load_interface_templates(None).unwrap();
        let yaml = r#"
namespace: test::ns

structs:
  - name: MyStruct
    top_level: true
    members:
      - name: value
        type: uint32_t
        default: 42
      - name: name
        type: string
      - name: data
        type: uint8_t[1024]
"#;
        let list = InterfaceList::from_str(yaml).unwrap();
        let ctx = InterfaceStructContext::from(&list.namespace, &list.structs[0]);
        let content = render(&templates.interface_struct, &ctx).unwrap();

        assert!(content.contains("#ifndef TEST_NS_MY_STRUCT_HPP"));
        assert!(content.contains("#define TEST_NS_MY_STRUCT_HPP"));
        assert!(content.contains("namespace test::ns {"));
        assert!(content.contains("struct MyStruct {"));
        assert!(content.contains("uint32_t value{42};"));
        assert!(content.contains("std::string name;"));
        assert!(content.contains("std::array<uint8_t, 1024> data;"));
        assert!(content.contains("#include <array>"));
        assert!(content.contains("#include <cstdint>"));
        assert!(content.contains("#include <string>"));
        assert!(content.contains("#endif // TEST_NS_MY_STRUCT_HPP"));
    }

    #[test]
    fn test_render_struct_with_custom_types() {
        let templates = load_interface_templates(None).unwrap();
        let yaml = r#"
namespace: adas::perception

structs:
  - name: DetectedObject
    top_level: false
    members:
      - name: object_class
        type: ObjectClass
      - name: bounding_box
        type: BoundingBox
      - name: confidence
        type: float
        default: 0.0
"#;
        let list = InterfaceList::from_str(yaml).unwrap();
        let ctx = InterfaceStructContext::from(&list.namespace, &list.structs[0]);
        let content = render(&templates.interface_struct, &ctx).unwrap();

        assert!(content.contains("#include \"bounding_box.hpp\""));
        assert!(content.contains("#include \"object_class.hpp\""));
        assert!(content.contains("ObjectClass object_class;"));
        assert!(content.contains("BoundingBox bounding_box;"));
        assert!(content.contains("float confidence{0.0f};"));
    }

    #[test]
    fn test_render_enum_template() {
        let templates = load_interface_templates(None).unwrap();
        let yaml = r#"
namespace: test

enums:
  - name: Color
    base: uint8_t
    values:
      - name: kRed
        value: 0
      - name: kGreen
        value: 1
      - name: kBlue
        value: 2
"#;
        let list = InterfaceList::from_str(yaml).unwrap();
        let ctx = InterfaceEnumContext::from(&list.namespace, &list.enums[0]);
        let content = render(&templates.interface_enum, &ctx).unwrap();

        assert!(content.contains("#ifndef TEST_COLOR_HPP"));
        assert!(content.contains("namespace test {"));
        assert!(content.contains("enum class Color : uint8_t {"));
        assert!(content.contains("kRed = 0,"));
        assert!(content.contains("kGreen = 1,"));
        assert!(content.contains("kBlue = 2,"));
        assert!(content.contains("#include <cstdint>"));
    }

    #[test]
    fn test_render_constexpr_template() {
        let templates = load_interface_templates(None).unwrap();
        let yaml = r#"
namespace: adas::perception

constexprs:
  - name: MaxSpeed
    member_type: float
    value: 150.0
  - name: FrameRate
    member_type: uint32_t
    value: 30
"#;
        let list = InterfaceList::from_str(yaml).unwrap();

        let ctx0 = InterfaceConstexprContext::from(&list.namespace, &list.constexprs[0]);
        let content0 = render(&templates.interface_constexpr, &ctx0).unwrap();
        assert!(content0.contains("constexpr float MaxSpeed = 150.0f;"));
        assert!(!content0.contains("#include <cstdint>"));

        let ctx1 = InterfaceConstexprContext::from(&list.namespace, &list.constexprs[1]);
        let content1 = render(&templates.interface_constexpr, &ctx1).unwrap();
        assert!(content1.contains("constexpr uint32_t FrameRate = 30;"));
        assert!(content1.contains("#include <cstdint>"));
    }

    #[test]
    fn test_generate_interfaces_creates_files() {
        let dir = std::env::temp_dir().join("scoregen_test_interfaces");
        let _ = fs::remove_dir_all(&dir);

        let yaml = r#"
namespace: test::ns

structs:
  - name: MyStruct
    top_level: true
    members:
      - name: x
        type: uint32_t
        default: 0

enums:
  - name: MyEnum
    base: uint8_t
    values:
      - name: kA
        value: 0

constexprs:
  - name: MaxVal
    member_type: float
    value: 100.0
"#;
        let list = InterfaceList::from_str(yaml).unwrap();
        let generated = generate_interfaces(&dir, &[list], None).unwrap();

        assert_eq!(generated.len(), 3);
        assert!(dir.join("test/ns/my_struct.hpp").exists());
        assert!(dir.join("test/ns/my_enum.hpp").exists());
        assert!(dir.join("test/ns/max_val.hpp").exists());

        let struct_hpp = fs::read_to_string(dir.join("test/ns/my_struct.hpp")).unwrap();
        assert!(struct_hpp.contains("struct MyStruct"));
        assert!(struct_hpp.contains("namespace test::ns"));

        let enum_hpp = fs::read_to_string(dir.join("test/ns/my_enum.hpp")).unwrap();
        assert!(enum_hpp.contains("enum class MyEnum"));

        let constexpr_hpp = fs::read_to_string(dir.join("test/ns/max_val.hpp")).unwrap();
        assert!(constexpr_hpp.contains("constexpr float MaxVal"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generate_interfaces_namespace_directory_structure() {
        let dir = std::env::temp_dir().join("scoregen_test_iface_ns");
        let _ = fs::remove_dir_all(&dir);

        let yaml = r#"
namespace: adas::perception

structs:
  - name: CameraFrame
    top_level: true
    members:
      - name: width
        type: uint32_t
        default: 1920
"#;
        let list = InterfaceList::from_str(yaml).unwrap();
        let generated = generate_interfaces(&dir, &[list], None).unwrap();

        assert_eq!(generated, vec!["adas::perception::CameraFrame"]);
        assert!(dir.join("adas/perception/camera_frame.hpp").exists());

        let _ = fs::remove_dir_all(&dir);
    }
}

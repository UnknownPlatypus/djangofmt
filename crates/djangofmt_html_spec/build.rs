//! Build script for `djangofmt_html_spec`.
//!
//! This script runs at compile time to:
//! 1. Fetch the HTML spec from markuplint's CDN
//! 2. Load local HTMX and Alpine.js attribute definitions
//! 3. Generate Rust code with `phf` perfect hash maps for O(1) lookups
//!
//! The generated code is written to `$OUT_DIR/generated_specs.rs` and included
//! via `include!()` in `lib.rs`.

#![allow(
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::collapsible_if
)]

use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use serde::Deserialize;

const HTML_SPEC_URL: &str = "https://cdn.jsdelivr.net/npm/@markuplint/html-spec@latest/index.json";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=data/htmx-attrs.json");
    println!("cargo:rerun-if-changed=data/alpine-attrs.json");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_specs.rs");
    let mut file = BufWriter::new(File::create(&dest_path).unwrap());

    // Fetch HTML spec from CDN
    let html_spec: MarkuplintSpec = fetch_json(HTML_SPEC_URL);

    // Load local HTMX and Alpine specs
    let htmx_spec: LocalSpec =
        serde_json::from_str(&fs::read_to_string("data/htmx-attrs.json").unwrap()).unwrap();
    let alpine_spec: LocalSpec =
        serde_json::from_str(&fs::read_to_string("data/alpine-attrs.json").unwrap()).unwrap();

    // Generate code
    generate_elements(&mut file, &html_spec);
    generate_global_attrs(&mut file, &html_spec, &htmx_spec, &alpine_spec);
}

fn fetch_json<T: for<'de> Deserialize<'de>>(url: &str) -> T {
    let response = ureq::get(url).call().expect("Failed to fetch HTML spec");
    response
        .into_json()
        .expect("Failed to parse HTML spec JSON")
}

fn generate_elements(file: &mut impl Write, spec: &MarkuplintSpec) {
    // Types are imported by the wrapper module in lib.rs

    // Build element map
    let mut element_map = phf_codegen::Map::new();

    for element in &spec.specs {
        let name = element.name.to_ascii_lowercase();
        let deprecated = element.deprecated.unwrap_or(false);
        let void_element = element.void.unwrap_or(false);

        // Collect element-specific attributes
        let attrs = collect_element_attrs(element);

        let attrs_code = if attrs.is_empty() {
            "&[]".to_string()
        } else {
            format!("&[{}]", attrs.join(", "))
        };

        let element_code = format!(
            "ElementSpec {{ name: {:?}, deprecated: {}, void_element: {}, attributes: {} }}",
            name, deprecated, void_element, attrs_code
        );

        element_map.entry(name, &element_code);
    }

    writeln!(
        file,
        "/// HTML element specifications.\npub static ELEMENTS: phf::Map<&'static str, ElementSpec> = {};",
        element_map.build()
    )
    .unwrap();
    writeln!(file).unwrap();
}

fn collect_element_attrs(element: &ElementDef) -> Vec<String> {
    let mut attrs = Vec::new();

    if let Some(ref attributes) = element.attributes {
        for (name, attr_def) in attributes {
            if name.starts_with('#') {
                continue; // Skip references like #globalAttrs
            }

            let deprecated = attr_def.deprecated.unwrap_or(false);
            let value_type = convert_attr_type(&attr_def.attr_type);

            attrs.push(format!(
                "AttributeSpec {{ name: {:?}, deprecated: {}, value_type: {} }}",
                name.to_ascii_lowercase(),
                deprecated,
                value_type
            ));
        }
    }

    attrs
}

fn generate_global_attrs(
    file: &mut impl Write,
    html_spec: &MarkuplintSpec,
    htmx_spec: &LocalSpec,
    alpine_spec: &LocalSpec,
) {
    let mut global_map = phf_codegen::Map::new();

    // HTML global attrs from def section
    if let Some(ref def) = html_spec.def {
        if let Some(ref global_attrs) = def.html_global_attrs {
            for (name, attr_def) in global_attrs {
                if name.starts_with('#') {
                    continue;
                }
                let deprecated = attr_def.deprecated.unwrap_or(false);
                let value_type = convert_attr_type(&attr_def.attr_type);

                let attr_code =
                    format_attr_spec(&name.to_ascii_lowercase(), deprecated, &value_type);
                global_map.entry(name.to_ascii_lowercase(), &attr_code);
            }
        }
    }

    // HTMX global attrs
    for (name, attr_def) in &htmx_spec.global_attrs {
        let deprecated = attr_def.deprecated.unwrap_or(false);
        let value_type = convert_local_attr_type(&attr_def.attr_type);
        let attr_code = format_attr_spec(&name.to_ascii_lowercase(), deprecated, &value_type);
        global_map.entry(name.to_ascii_lowercase(), &attr_code);
    }

    // Alpine global attrs
    for (name, attr_def) in &alpine_spec.global_attrs {
        let deprecated = attr_def.deprecated.unwrap_or(false);
        let value_type = convert_local_attr_type(&attr_def.attr_type);
        let attr_code = format_attr_spec(&name.to_ascii_lowercase(), deprecated, &value_type);
        global_map.entry(name.to_ascii_lowercase(), &attr_code);
    }

    writeln!(
        file,
        "/// Global HTML attributes (including HTMX and Alpine.js).\npub static GLOBAL_ATTRS: phf::Map<&'static str, AttributeSpec> = {};",
        global_map.build()
    )
    .unwrap();
}

/// Format an `AttributeSpec` struct literal.
fn format_attr_spec(name: &str, deprecated: bool, value_type: &str) -> String {
    format!(
        "AttributeSpec {{ name: {:?}, deprecated: {}, value_type: {} }}",
        name, deprecated, value_type
    )
}

/// Convert markuplint's attribute type to our `AttributeValueType` code.
fn convert_attr_type(attr_type: &Option<AttrType>) -> String {
    match attr_type {
        None => "AttributeValueType::Any".to_string(),
        Some(AttrType::String(s)) => match s.as_str() {
            "Boolean" => "AttributeValueType::Boolean".to_string(),
            "URL" | "AbsoluteURL" => "AttributeValueType::Url".to_string(),
            "Int" | "Integer" => "AttributeValueType::Integer".to_string(),
            "Uint" | "NonNegativeInteger" => "AttributeValueType::PositiveInteger".to_string(),
            "Number" | "Float" => "AttributeValueType::Number".to_string(),
            _ => "AttributeValueType::Any".to_string(),
        },
        Some(AttrType::Object(obj)) => {
            if let Some(ref values) = obj.enum_values {
                let escaped: Vec<String> = values.iter().map(|v| format!("{:?}", v)).collect();
                format!("AttributeValueType::Enum(&[{}])", escaped.join(", "))
            } else {
                "AttributeValueType::Any".to_string()
            }
        }
        Some(AttrType::Other(_)) => "AttributeValueType::Any".to_string(),
    }
}

/// Convert local (HTMX/Alpine) attribute type to our `AttributeValueType` code.
fn convert_local_attr_type(attr_type: &LocalAttrType) -> String {
    match attr_type {
        LocalAttrType::String(s) => match s.as_str() {
            "Boolean" => "AttributeValueType::Boolean".to_string(),
            "URL" => "AttributeValueType::Url".to_string(),
            "Integer" | "Int" => "AttributeValueType::Integer".to_string(),
            _ => "AttributeValueType::Any".to_string(),
        },
        LocalAttrType::Object(obj) => {
            if let Some(ref values) = obj.enum_values {
                let escaped: Vec<String> = values.iter().map(|v| format!("{:?}", v)).collect();
                format!("AttributeValueType::Enum(&[{}])", escaped.join(", "))
            } else {
                "AttributeValueType::Any".to_string()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Serde types for markuplint HTML spec
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct MarkuplintSpec {
    #[serde(default)]
    def: Option<DefSection>,
    #[serde(default)]
    specs: Vec<ElementDef>,
}

#[derive(Debug, Deserialize)]
struct DefSection {
    #[serde(rename = "#HTMLGlobalAttrs")]
    html_global_attrs: Option<HashMap<String, AttrDef>>,
}

#[derive(Debug, Deserialize)]
struct ElementDef {
    name: String,
    #[serde(default)]
    deprecated: Option<bool>,
    #[serde(rename = "void", default)]
    void: Option<bool>,
    #[serde(default)]
    attributes: Option<HashMap<String, AttrDef>>,
}

#[derive(Debug, Deserialize)]
struct AttrDef {
    #[serde(rename = "type")]
    attr_type: Option<AttrType>,
    #[serde(default)]
    deprecated: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AttrType {
    String(String),
    Object(AttrTypeObject),
    // Complex types we don't handle yet (arrays, nested objects, etc.)
    #[allow(dead_code)]
    Other(serde_json::Value),
}

#[derive(Debug, Deserialize)]
struct AttrTypeObject {
    #[serde(rename = "enum")]
    enum_values: Option<Vec<String>>,
}

// Serde types for local HTMX/Alpine specs

#[derive(Debug, Deserialize)]
struct LocalSpec {
    #[serde(rename = "globalAttrs")]
    global_attrs: HashMap<String, LocalAttrDef>,
}

#[derive(Debug, Deserialize)]
struct LocalAttrDef {
    #[serde(rename = "type")]
    attr_type: LocalAttrType,
    #[serde(default)]
    deprecated: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LocalAttrType {
    String(String),
    Object(LocalAttrTypeObject),
}

#[derive(Debug, Deserialize)]
struct LocalAttrTypeObject {
    #[serde(rename = "enum")]
    enum_values: Option<Vec<String>>,
}

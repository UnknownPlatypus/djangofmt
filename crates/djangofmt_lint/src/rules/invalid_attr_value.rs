//! E002: Invalid attribute value.
//!
//! Validates attribute values against the HTML specification.
//! Currently only validates enum-type attributes.

use djangofmt_html_spec::{AttributeValueType, ELEMENTS, GLOBAL_ATTRS};
use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;

#[inline]
fn contains_interpolation(value: &str) -> bool {
    value.contains("{{") || value.contains("{%")
}

pub fn check(element: &Element<'_>, checker: &mut Checker<'_>) {
    let tag = element.tag_name.to_ascii_lowercase();

    // Look up element spec (skip unknown elements like web components)
    let element_spec = ELEMENTS.get(tag.as_str());

    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value, .. }) = attr {
            let attr_name = name.to_ascii_lowercase();

            // Skip if no value
            let Some((value_str, offset)) = value else {
                continue;
            };

            // Skip interpolated values
            if contains_interpolation(value_str) {
                continue;
            }

            // Find attribute spec: first check element-specific, then global
            let attr_spec = element_spec
                .and_then(|e| e.attributes.iter().find(|a| a.name == attr_name))
                .or_else(|| GLOBAL_ATTRS.get(attr_name.as_str()));

            let Some(spec) = attr_spec else {
                continue; // Unknown attribute, skip
            };

            // Validate enum values
            if let AttributeValueType::Enum(allowed) = &spec.value_type {
                let normalized = value_str.to_ascii_lowercase();
                if !allowed.iter().any(|v| v.eq_ignore_ascii_case(&normalized)) {
                    let allowed_list = allowed.join(", ");
                    checker.report(
                        "E002",
                        format!(
                            "Invalid value '{value_str}' for attribute '{name}'. Expected one of: {allowed_list}"
                        ),
                        (*offset, value_str.len()).into(),
                        "invalid value".into(),
                        Some(format!("Use one of: {allowed_list}")),
                    );
                }
            }
        }
    }
}

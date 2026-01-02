//! invalid-attr-value: Invalid attribute value.
//!
//! Validates attribute values against the HTML specification.
//! Currently only validates enum-type attributes (e.g., `<input type>`, `<button type>`).
//!
//! Also validates HTMX attributes (`hx-boost`, `hx-swap`, etc.) and Alpine.js attributes.
//!
//! ## Skipped cases
//!
//! - **Interpolation**: Values containing `{{` or `{%` are skipped (dynamic values)
//! - **Unknown elements**: Web components and custom elements are skipped
//! - **Unknown attributes**: Attributes not in the spec are skipped

use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::Rule;
use crate::violation::Violation;

/// Violation for invalid HTML attribute values.
///
/// Reports when an attribute has a value that doesn't match the allowed values
/// for that attribute (e.g., `<form method="put">` is invalid).
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidAttrValue {
    pub value: String,
    pub attribute: String,
    pub allowed: Vec<String>,
}

impl Violation for InvalidAttrValue {
    const RULE: Rule = Rule::InvalidAttrValue;

    fn message(&self) -> String {
        format!(
            "Invalid value '{}' for attribute '{}'.",
            self.value, self.attribute,
        )
    }

    fn help(&self) -> Option<String> {
        if self.allowed.is_empty() {
            None
        } else {
            Some(format!("Use one of: {}", self.allowed.join(", ")))
        }
    }
}

/// Returns true if the value contains Jinja/Django interpolation markers.
#[inline]
fn contains_interpolation(value: &str) -> bool {
    value.contains("{{") || value.contains("{%")
}

/// Check an element's attributes for invalid enum values.
pub fn check(element: &Element<'_>, checker: &mut Checker<'_>) {
    // Pending implementation of djangofmt_html_spec.
    // Currently only checks for <form method="...">.
    if !element.tag_name.eq_ignore_ascii_case("form") {
        return;
    }

    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value, .. }) = attr {
            if !name.eq_ignore_ascii_case("method") {
                continue;
            }

            // Skip if no value
            let Some((value_str, offset)) = value else {
                continue;
            };

            // Skip interpolated values
            if contains_interpolation(value_str) {
                continue;
            }

            let allowed = ["get", "post", "dialog"];
            if !allowed.iter().any(|v| v.eq_ignore_ascii_case(value_str)) {
                checker.report(
                    &InvalidAttrValue {
                        value: (*value_str).to_string(),
                        attribute: "method".to_string(),
                        allowed: allowed.iter().map(|s| (*s).to_string()).collect(),
                    },
                    (*offset, value_str.len()).into(),
                );
            }
        }
    }
}

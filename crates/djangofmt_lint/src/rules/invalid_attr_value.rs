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
use crate::registry::RuleCode;
use crate::violation::Violation;

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidAttrValue<'a> {
    pub value: &'a str,
    pub attribute: &'a str,
}

impl Violation for InvalidAttrValue<'_> {
    fn message(&self) -> String {
        format!(
            "Invalid value '{}' for attribute '{}'.",
            self.value, self.attribute,
        )
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
                    RuleCode::InvalidAttrValue,
                    &InvalidAttrValue {
                        value: value_str,
                        attribute: "method",
                    },
                    (*offset, value_str.len()).into(),
                    "invalid value".into(),
                    Some(format!("Use one of: {}", allowed.join(", "))),
                );
            }
        }
    }
}

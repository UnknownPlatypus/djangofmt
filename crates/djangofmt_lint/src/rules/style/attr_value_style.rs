//! attr-value-style: Stylistic rules for HTML attribute values.
//!
//! ## Rules
//!
//! - **H029 / uppercase-form-method**: Detects non-lowercase `method` values on `<form>`.
//! - **H033 / form-action-whitespace**: Detects leading/trailing whitespace in `<form action>`.
//! - **H026 / empty-attr-value**: Detects empty `id` or `class` attribute values.
//!
//! ## Skipped cases
//!
//! - **Interpolation**: Values containing `{{` or `{%` are skipped (dynamic values)

use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::Violation;

/// Violation for non-lowercase `method` attribute on `<form>`.
///
/// Reports when a `<form method="...">` attribute value is not lowercase.
#[derive(Debug, PartialEq, Eq)]
pub struct UppercaseFormMethod {
    pub value: String,
}

impl Violation for UppercaseFormMethod {
    const RULE: Rule = Rule::UppercaseFormMethod;
    const CATEGORY: RuleCategory = RuleCategory::Style;

    fn message(&self) -> String {
        format!("Form method '{}' should be lowercase.", self.value)
    }

    fn help(&self) -> Option<String> {
        Some(format!(
            "Use '{}' instead.",
            self.value.to_ascii_lowercase()
        ))
    }
}

/// Violation for leading/trailing whitespace in `<form action>`.
///
/// Reports when a `<form action="...">` attribute value has leading or trailing whitespace.
#[derive(Debug, PartialEq, Eq)]
pub struct FormActionWhitespace;

impl Violation for FormActionWhitespace {
    const RULE: Rule = Rule::FormActionWhitespace;
    const CATEGORY: RuleCategory = RuleCategory::Style;

    fn message(&self) -> String {
        "Extra whitespace found in form action.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Remove leading and trailing whitespace from the action URL.".to_string())
    }
}

/// Violation for empty `id` or `class` attribute values.
///
/// Reports when an `id=""` or `class=""` attribute has an empty value.
#[derive(Debug, PartialEq, Eq)]
pub struct EmptyAttrValue {
    pub attr: String,
}

impl Violation for EmptyAttrValue {
    const RULE: Rule = Rule::EmptyAttrValue;
    const CATEGORY: RuleCategory = RuleCategory::Style;

    fn message(&self) -> String {
        format!("Empty '{}' attribute can be removed.", self.attr)
    }
}

/// Check `<form method="...">` for non-lowercase values.
pub fn check_uppercase_form_method(element: &Element<'_>, checker: &mut Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("form") {
        return;
    }

    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value, .. }) = attr {
            if !name.eq_ignore_ascii_case("method") {
                continue;
            }

            let Some((value_str, offset)) = value else {
                continue;
            };

            if contains_interpolation(value_str) {
                continue;
            }

            if value_str.chars().any(|c| c.is_ascii_uppercase()) {
                checker.report(
                    &UppercaseFormMethod {
                        value: (*value_str).to_string(),
                    },
                    (*offset, value_str.len()).into(),
                );
            }
        }
    }
}

/// Check `<form action="...">` for leading/trailing whitespace.
pub fn check_form_action_whitespace(element: &Element<'_>, checker: &mut Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("form") {
        return;
    }

    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value, .. }) = attr {
            if !name.eq_ignore_ascii_case("action") {
                continue;
            }

            let Some((value_str, offset)) = value else {
                continue;
            };

            if contains_interpolation(value_str) {
                continue;
            }

            if *value_str != value_str.trim() {
                checker.report(&FormActionWhitespace, (*offset, value_str.len()).into());
            }
        }
    }
}

/// Check any element for empty `id` or `class` attribute values.
pub fn check_empty_attr_value(element: &Element<'_>, checker: &mut Checker<'_>) {
    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value, .. }) = attr {
            let is_id_or_class =
                name.eq_ignore_ascii_case("id") || name.eq_ignore_ascii_case("class");

            if !is_id_or_class {
                continue;
            }

            let Some((value_str, offset)) = value else {
                continue;
            };

            if value_str.is_empty() {
                checker.report(
                    &EmptyAttrValue {
                        attr: (*name).to_string(),
                    },
                    (*offset, value_str.len()).into(),
                );
            }
        }
    }
}

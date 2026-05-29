use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for non-lowercase `method` attribute values on `<form>` elements.
///
/// ## Why is this bad?
/// HTML form `method` values are case-insensitive, but the HTML spec and
/// conventional usage write them in lowercase (`get`, `post`, `dialog`).
/// Uppercase or mixed-case values are stylistically inconsistent.
///
/// Values containing template interpolation are skipped.
///
/// ## Example
/// ```html
/// <form method="POST"></form>
/// ```
///
/// Use instead:
/// ```html
/// <form method="post"></form>
/// ```
///
/// ## Fix safety
/// This rule's fix is marked as safe: the HTML spec defines form `method` as a
/// case-insensitive enumerated attribute, so lowercasing preserves runtime
/// semantics.
///
/// ## References
/// - [HTML spec: form submission method](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#attr-fs-method)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.9")]
pub struct UppercaseFormMethod<'a> {
    pub value: &'a str,
}

impl Violation for UppercaseFormMethod<'_> {
    const RULE: Rule = Rule::UppercaseFormMethod;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Form method `{}` should be lowercase.", self.value)
    }

    fn help(&self) -> Option<String> {
        Some(format!(
            "Use `{}` instead.",
            self.value.to_ascii_lowercase()
        ))
    }

    fn fix_title(&self) -> Option<String> {
        Some("Lowercase `method` value".to_string())
    }
}

pub fn check(element: &Element<'_>, checker: &Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("form") {
        return;
    }

    for attr in &element.attrs {
        let Attribute::Native(NativeAttribute { name, value, .. }) = attr else {
            continue;
        };

        if !name.eq_ignore_ascii_case("method") {
            continue;
        }

        let Some((value_str, offset)) = value else {
            continue;
        };

        if contains_interpolation(value_str) {
            continue;
        }

        if !value_str.chars().any(|c| c.is_ascii_uppercase()) {
            continue;
        }

        let mut guard = checker.report_diagnostic(
            &UppercaseFormMethod { value: value_str },
            (*offset, value_str.len()).into(),
        );

        guard.set_fix(Fix::safe_edit(Edit::replacement(
            value_str.to_ascii_lowercase(),
            (*offset, value_str.len()).into(),
        )));
    }
}

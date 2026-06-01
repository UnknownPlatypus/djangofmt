use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for HTML attributes whose value is not in the set allowed by the
/// HTML specification (and supported framework dialects such as HTMX or
/// Alpine.js).
///
/// Currently only validates enum-type attributes (e.g., `<form method>`,
/// `<input type>`, `<button type>`).
///
/// ## Why is this bad?
/// Browsers silently ignore unknown values for enum attributes and fall back
/// to a default, which usually does not match the author's intent. The
/// resulting bug is easy to miss because the page still renders.
///
/// Values containing template interpolation (`{{ ... }}` or `{% ... %}`),
/// unknown elements (web components, custom tags), and unknown attributes
/// are skipped.
///
/// ## Example
///
/// ```html
/// <form method="put"></form>
/// ```
///
/// Use instead:
///
/// ```html
/// <form method="post"></form>
/// ```
///
/// ## References
/// - [HTML Living Standard: `form.method`](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#attr-fs-method)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.5")]
pub struct InvalidAttrValue {
    pub value: String,
    pub attribute: &'static str,
    pub allowed: &'static [&'static str],
}

impl Violation for InvalidAttrValue {
    const RULE: Rule = Rule::InvalidAttrValue;
    const CATEGORY: RuleCategory = RuleCategory::Correctness;

    #[derive_message_formats]
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

/// Per-attribute check driven by the centralized element dispatcher.
///
/// Pending implementation of `djangofmt_html_spec`, this currently only validates
/// `<form method="...">`. The dispatcher pre-filters to a `method` attribute on
/// a `<form>` tag.
pub fn check_method_attr(checker: &Checker<'_>, value_str: &str, offset: usize) {
    // Skip interpolated values
    if contains_interpolation(value_str) {
        return;
    }

    let allowed: &[&str] = &["get", "post", "dialog"];
    if !allowed.iter().any(|v| v.eq_ignore_ascii_case(value_str)) {
        checker.report_diagnostic(
            &InvalidAttrValue {
                value: value_str.to_string(),
                attribute: "method",
                allowed,
            },
            (offset, value_str.len()).into(),
        );
    }
}

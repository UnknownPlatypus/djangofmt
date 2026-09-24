use std::borrow::Cow;

use markup_fmt::ast::{Element, NativeAttribute};

use crate::Checker;
use crate::html_spec::enum_attr;
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::{closest_match, contains_interpolation};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for HTML attributes whose value is not one of the keywords the HTML specification allows,
/// such as `<form method>`, `<input type>` or `<img loading>`.
///
/// ## Why is this bad?
/// Browsers silently ignore an unknown keyword and fall back to a default, which usually does not
/// match the author's intent: `<form method="put">` sends a `GET` request and
/// `<img loading="lazzy">` loads eagerly. The resulting bug is easy to miss because the page still
/// renders.
///
/// Keywords match case-insensitively, except where the specification makes case significant, as in
/// `<ol type>`. Values containing template interpolation (`{{ ... }}` or `{% ... %}`) and custom
/// elements are skipped.
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
/// - [HTML Living Standard: Keywords and enumerated attributes](https://html.spec.whatwg.org/multipage/common-microsyntaxes.html#keywords-and-enumerated-attributes)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.5")]
pub struct InvalidAttrValue {
    pub value: String,
    pub attribute: String,
    pub allowed: &'static [&'static str],
    pub suggestion: Option<&'static str>,
}

impl Violation for InvalidAttrValue {
    const RULE: Rule = Rule::InvalidAttrValue;
    const CATEGORY: RuleCategory = RuleCategory::Correctness;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        format!(
            "Invalid value `{}` for attribute `{}`",
            self.value, self.attribute,
        )
        .into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        if let Some(keyword) = self.suggestion {
            return Some(format!("Did you mean `{keyword}`?").into());
        }
        let allowed: Vec<_> = self
            .allowed
            .iter()
            .map(|&keyword| if keyword.is_empty() { "\"\"" } else { keyword })
            .collect();
        Some(format!("Use one of: {}", allowed.join(", ")).into())
    }
}

/// Check a single attribute for a value outside its keywords.
pub fn check(checker: &Checker<'_>, attr: &NativeAttribute<'_>, element: &Element<'_>) {
    let NativeAttribute {
        name,
        value: Some((value, _)),
        ..
    } = attr
    else {
        return;
    };
    let Some(spec) = enum_attr(element.tag_name, name) else {
        return;
    };
    if spec.accepts(value) || contains_interpolation(value) {
        return;
    }

    let typed = if spec.case_insensitive {
        Cow::Owned(value.to_ascii_lowercase())
    } else {
        Cow::Borrowed(*value)
    };
    let keywords = spec
        .values
        .iter()
        .copied()
        .filter(|keyword| !keyword.is_empty());
    checker.report_diagnostic(
        &InvalidAttrValue {
            value: (*value).into(),
            attribute: (*name).into(),
            allowed: spec.values,
            suggestion: closest_match(&typed, keywords),
        },
        checker.source_span(value),
    );
}

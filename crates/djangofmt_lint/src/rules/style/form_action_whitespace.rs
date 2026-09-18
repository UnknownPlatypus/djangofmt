use std::borrow::Cow;

use markup_fmt::ast::NativeAttribute;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for leading or trailing whitespace in the `action` attribute of `<form>` elements.
///
/// ## Why is this bad?
/// The URL parser strips leading and trailing ASCII whitespace when resolving an HTML URL
/// attribute, so the spaces are inert at runtime and only add noise to the source. They are
/// commonly an accidental artefact of inserting a template tag inside the quotes.
///
/// Padding around a template tag is reported too: whatever the tag renders, the browser strips
/// the edges of the final value, so trimming the source is always safe. Whitespace next to a
/// whitespace-control marker (`{%-`, `-%}`) is already removed by the template engine and is
/// left alone. Only the `action` attribute is checked; sibling attributes such as `data-action`
/// may legitimately span multiple lines.
///
/// ## Example
/// ```html
/// <form action=" /submit/ "></form>
/// ```
///
/// Use instead:
/// ```html
/// <form action="/submit/"></form>
/// ```
///
/// ## References
/// - [URL Standard: basic URL parser](https://url.spec.whatwg.org/#concept-basic-url-parser)
/// - [HTML spec: the form element](https://html.spec.whatwg.org/multipage/forms.html#the-form-element)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.9")]
pub struct FormActionWhitespace;

impl Violation for FormActionWhitespace {
    const RULE: Rule = Rule::FormActionWhitespace;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        "Extra whitespace in form `action`".into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some("Trim the `action` value".into())
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Trim whitespace from `action` value")
    }
}

/// The caller guarantees the attribute belongs to a `<form>`.
pub fn check(checker: &Checker<'_>, attr: &NativeAttribute<'_>) {
    let NativeAttribute {
        name,
        value: Some((value_str, _)),
        ..
    } = attr
    else {
        return;
    };

    if !name.eq_ignore_ascii_case("action") {
        return;
    }

    let start = value_str.trim_ascii_start();
    let trimmed = start.trim_ascii_end();
    // Whitespace-control markers already strip the padding next to them.
    let leading_inert =
        start.len() == value_str.len() || start.starts_with("{%-") || start.starts_with("{{-");
    let trailing_inert =
        trimmed.len() == start.len() || trimmed.ends_with("-%}") || trimmed.ends_with("-}}");
    if leading_inert && trailing_inert {
        return;
    }

    let span = checker.source_span(value_str);
    let mut guard = checker.report_diagnostic(&FormActionWhitespace, span);

    let edit = if trimmed.is_empty() {
        Edit::deletion(span)
    } else {
        Edit::replacement(trimmed.to_string(), span)
    };
    guard.set_fix(Fix::safe_edit(edit));
}

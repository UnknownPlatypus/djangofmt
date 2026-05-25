use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for leading or trailing whitespace in the `action` attribute of `<form>` elements.
///
/// ## Why is this bad?
/// The URL parser strips leading and trailing ASCII whitespace when resolving an HTML URL
/// attribute, so the spaces are inert at runtime and only add noise to the source. They are
/// commonly an accidental artefact of inserting a template tag inside the quotes.
///
/// Values containing template interpolation are skipped: the surrounding whitespace is usually
/// intentional padding around a `{% url %}` tag and cannot be safely trimmed without knowing the
/// rendered output. Only the `action` attribute is checked; sibling attributes such as
/// `data-action` may legitimately span multiple lines.
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
/// ## Fix safety
/// This rule's fix is marked as safe: the URL parser strips leading and trailing ASCII
/// whitespace from URL attribute values, so trimming the literal source preserves runtime
/// semantics. Author code reading the raw attribute (e.g. `form.getAttribute("action")`,
/// strict-equality dispatch, HTMX or web-component integrations that observe the literal
/// string) would see the trimmed value, but relying on the surrounding whitespace is
/// vanishingly rare.
///
/// ## References
/// - [URL Standard: basic URL parser](https://url.spec.whatwg.org/#concept-basic-url-parser)
/// - [HTML spec: the form element](https://html.spec.whatwg.org/multipage/forms.html#the-form-element)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct FormActionWhitespace;

impl Violation for FormActionWhitespace {
    const RULE: Rule = Rule::FormActionWhitespace;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> String {
        "Extra whitespace found in form `action`.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Remove leading and trailing whitespace from the `action` value.".to_string())
    }

    fn fix_title(&self) -> Option<String> {
        Some("Trim whitespace from `action` value".to_string())
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

        if !name.eq_ignore_ascii_case("action") {
            continue;
        }

        let Some((value_str, offset)) = value else {
            continue;
        };

        if contains_interpolation(value_str) {
            continue;
        }

        let trimmed = value_str.trim_ascii();
        if trimmed.len() == value_str.len() {
            continue;
        }

        let span = (*offset, value_str.len()).into();
        let mut guard = checker.report_diagnostic(&FormActionWhitespace, span);

        let edit = if trimmed.is_empty() {
            Edit::deletion(span)
        } else {
            Edit::replacement(trimmed.to_string(), span)
        };
        guard.set_fix(Fix::safe_edit(edit));
    }
}

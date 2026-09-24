use std::borrow::Cow;

use markup_fmt::ast::{Element, NativeAttribute};

use crate::Checker;
use crate::fix::FixAvailability;
use crate::fix::edits::delete_attr_fix;
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::is_custom_element;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for empty `id` or `class` attribute values on HTML elements.
///
/// ## Why is this bad?
/// An `id=""` or `class=""` attribute is almost always unintentional: no CSS class selector
/// matches an element with an empty `class`, and `document.getElementById("")` returns nothing.
/// Removing the attribute reduces template noise.
///
/// Custom elements (`<my-widget>`, `<c-button>`) are skipped: their attributes are the
/// component's API. For example, [django-cotton](https://django-cotton.com/docs/components#vars)
/// uses `class=""` to declare or override a component variable, so removing it changes the output.
///
/// ## Example
/// ```html
/// <div id="" class="">content</div>
/// ```
///
/// Use instead:
/// ```html
/// <div>content</div>
/// ```
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.9")]
pub struct EmptyAttrValue<'a> {
    pub attr: &'a str,
}

impl Violation for EmptyAttrValue<'_> {
    const RULE: Rule = Rule::EmptyAttrValue;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        format!("Empty `{}` attribute", self.attr).into()
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Remove empty attribute")
    }
}

pub fn check(checker: &Checker<'_>, attr: &NativeAttribute<'_>, element: &Element<'_>) {
    if is_custom_element(element.tag_name) {
        return;
    }

    let NativeAttribute {
        name,
        value: Some((value_str, _)),
        quote,
    } = attr
    else {
        return;
    };

    if !name.eq_ignore_ascii_case("id") && !name.eq_ignore_ascii_case("class") {
        return;
    }

    if !value_str.is_empty() {
        return;
    }

    let mut guard = checker.report_diagnostic(
        &EmptyAttrValue { attr: name },
        checker.source_span(value_str),
    );

    guard.set_fix(delete_attr_fix(
        checker.context(),
        name,
        value_str,
        quote.is_some(),
    ));
}

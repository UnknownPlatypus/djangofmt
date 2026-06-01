use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::reverse_consume_ws;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for empty `id` or `class` attribute values on HTML elements.
///
/// ## Why is this bad?
/// An `id=""` or `class=""` attribute is almost always unintentional: no CSS class selector
/// matches an element with an empty `class`, and `document.getElementById("")` returns nothing.
/// Removing the attribute reduces template noise.
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
///
/// ## Fix safety
/// The fix is marked as safe. In practice, removing an empty `id`/`class` preserves rendering
/// in every realistic template. The DOM technically distinguishes `<div id="">` from
/// `<div>` (`hasAttribute("id")`, the attribute selector `[id=""]`), but author code relying
/// on those forms is vanishingly rare.
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
    fn message(&self) -> String {
        format!("Empty `{}` attribute can be removed.", self.attr)
    }

    fn help(&self) -> Option<String> {
        Some(format!("Remove the empty `{}` attribute.", self.attr))
    }

    fn fix_title(&self) -> Option<String> {
        Some("Remove empty attribute".to_string())
    }
}

pub fn check(element: &Element<'_>, checker: &Checker<'_>) {
    for attr in &element.attrs {
        let Attribute::Native(NativeAttribute {
            name,
            value: Some((value_str, offset)),
            quote,
        }) = attr
        else {
            continue;
        };

        if !name.eq_ignore_ascii_case("id") && !name.eq_ignore_ascii_case("class") {
            continue;
        }

        if !value_str.is_empty() {
            continue;
        }

        let mut guard = checker.report_diagnostic(
            &EmptyAttrValue { attr: name },
            (*offset, value_str.len()).into(),
        );

        let name_start = checker.source_offset(name);
        let attr_end = *offset + value_str.len() + usize::from(quote.is_some());
        let fix_start = reverse_consume_ws(checker.context().source().as_bytes(), name_start);

        guard.set_fix(Fix::safe_edit(Edit::deletion(
            (fix_start, attr_end - fix_start).into(),
        )));
    }
}

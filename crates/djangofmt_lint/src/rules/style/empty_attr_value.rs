use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::violation::Violation;

/// ## What it does
/// Checks for empty `id` or `class` attribute values on HTML elements.
///
/// ## Why is this bad?
/// An `id=""` or `class=""` attribute has no effect: no selector matches the empty string and
/// no element can be referenced by an empty `id`. Removing the attribute reduces noise without
/// changing rendering or behaviour.
///
/// Only the `id` and `class` attributes are checked. Custom data attributes (`data-id`,
/// `data-foo-id-value`, etc.) and other attributes that legitimately accept an empty string
/// (such as `<form action="">`) are not flagged.
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
/// This rule's fix is marked as safe: removing an empty `id` or `class` attribute preserves
/// runtime semantics because no selector or document API can match an empty value.
#[derive(Debug, PartialEq, Eq)]
pub struct EmptyAttrValue<'a> {
    pub attr: &'a str,
}

impl Violation for EmptyAttrValue<'_> {
    const RULE: Rule = Rule::EmptyAttrValue;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

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
        let Attribute::Native(NativeAttribute { name, value, quote }) = attr else {
            continue;
        };

        if !name.eq_ignore_ascii_case("id") && !name.eq_ignore_ascii_case("class") {
            continue;
        }

        let Some((value_str, offset)) = value else {
            continue;
        };

        if !value_str.is_empty() {
            continue;
        }

        let mut guard = checker.report_diagnostic(
            &EmptyAttrValue { attr: name },
            (*offset, value_str.len()).into(),
        );

        let name_start = checker.source_offset(name);
        let attr_end = *offset + value_str.len() + usize::from(quote.is_some());

        // Absorb the whitespace separating this attribute from the previous
        // token so removing the only attribute leaves `<div>` rather than
        // `<div >`.
        let source = checker.context().source().as_bytes();
        let mut fix_start = name_start;
        while fix_start > 0 && source[fix_start - 1].is_ascii_whitespace() {
            fix_start -= 1;
        }

        guard.set_fix(Fix::safe_edit(Edit::deletion(
            (fix_start, attr_end - fix_start).into(),
        )));
    }
}

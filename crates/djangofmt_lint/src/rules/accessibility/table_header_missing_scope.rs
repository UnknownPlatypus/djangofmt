use markup_fmt::ast::Element;

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::declares_native_attr;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `<th>` elements that have no `scope` attribute.
///
/// ## Why is this bad?
/// The `scope` attribute tells assistive technology whether a header cell labels a column
/// (`scope="col"`) or a row (`scope="row"`). Without it, screen readers must guess the
/// header-to-data association in anything but the simplest table, so cells may be announced with the
/// wrong — or no — header. Explicit `scope` makes the data relationships unambiguous (WCAG 1.3.1,
/// Info and Relationships).
///
/// A `scope` declared inside any branch of a Jinja `{% if %}` block wrapping attributes also
/// counts as present.
///
/// ## Example
/// ```html
/// <table>
///     <tr><th>Name</th><th>Email</th></tr>
///     <tr><td>Ada</td><td>ada@example.com</td></tr>
/// </table>
/// ```
///
/// Use instead:
/// ```html
/// <table>
///     <tr><th scope="col">Name</th><th scope="col">Email</th></tr>
///     <tr><td>Ada</td><td>ada@example.com</td></tr>
/// </table>
/// ```
///
/// ## References
/// - [WCAG 1.3.1: Info and Relationships](https://www.w3.org/WAI/WCAG21/Understanding/info-and-relationships.html)
/// - [MDN: `<th>` `scope`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/th#scope)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct TableHeaderMissingScope;

impl Violation for TableHeaderMissingScope {
    const RULE: Rule = Rule::TableHeaderMissingScope;
    const CATEGORY: RuleCategory = RuleCategory::Accessibility;

    #[derive_message_formats]
    fn message(&self) -> String {
        "Missing `scope` attribute on `<th>`.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some(
            "Add `scope=\"col\"` or `scope=\"row\"` to associate the header with its data."
                .to_string(),
        )
    }
}

/// The caller guarantees `element` is a `<th>`.
pub fn check(element: &Element<'_>, checker: &Checker<'_>) {
    if element
        .attrs
        .iter()
        .any(|attr| declares_native_attr(attr, "scope"))
    {
        return;
    }

    let offset = checker.source_offset(element.tag_name);
    checker.report_diagnostic(
        &TableHeaderMissingScope,
        (offset, element.tag_name.len()).into(),
    );
}

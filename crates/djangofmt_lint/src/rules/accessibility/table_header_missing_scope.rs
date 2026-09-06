use std::borrow::Cow;

use markup_fmt::ast::{Attribute, Element};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::{contains_interpolation, declares_native_attr};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// The keywords a `scope` attribute may take (HTML spec / WCAG H63).
const VALID_SCOPE_VALUES: [&str; 4] = ["row", "col", "rowgroup", "colgroup"];

#[derive(Debug, PartialEq, Eq)]
pub enum ScopeViolation {
    /// No `scope` attribute, or one with an empty/valueless value.
    MissingOrEmpty,
    /// A `scope` attribute whose value is not one of `VALID_SCOPE_VALUES`.
    InvalidValue { value: String },
}

/// ## What it does
/// Checks that every `<th>` header cell has a `scope` attribute set to one of the valid keywords
/// `row`, `col`, `rowgroup`, or `colgroup`.
///
/// ## Why is this bad?
/// The `scope` attribute tells assistive technology whether a header cell labels a column or a row.
/// Without it, screen readers must guess the header-to-data association in anything but the simplest
/// table, so cells may be announced with the wrong header or none at all.
///
///
/// ## Example
/// ```html
/// <table>
///     <tr><th>Name</th><th scope="column">Email</th></tr>
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
/// - [WCAG H63: Using the `scope` attribute](https://www.w3.org/WAI/WCAG21/Techniques/html/H63)
/// - [MDN: `<th>` `scope`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/th#scope)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.12")]
pub struct TableHeaderMissingScope {
    pub kind: ScopeViolation,
}

impl Violation for TableHeaderMissingScope {
    const RULE: Rule = Rule::TableHeaderMissingScope;
    const CATEGORY: RuleCategory = RuleCategory::Accessibility;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        match &self.kind {
            ScopeViolation::MissingOrEmpty => "Missing or empty `scope` attribute on `<th>`".into(),
            ScopeViolation::InvalidValue { value } => {
                format!("Invalid `scope` value `{value}` on `<th>`").into()
            }
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some(match &self.kind {
            ScopeViolation::MissingOrEmpty => "Add `scope=\"col\"` or `scope=\"row\"`".into(),
            ScopeViolation::InvalidValue { .. } => {
                "Use one of `row`, `col`, `rowgroup`, or `colgroup`".into()
            }
        })
    }
}

/// The caller guarantees `element` is a `<th>`.
pub fn check(element: &Element<'_>, checker: &Checker<'_>) {
    let native_scope = element.attrs.iter().find_map(|attr| match attr {
        Attribute::Native(native) if native.name.eq_ignore_ascii_case("scope") => Some(native),
        _ => None,
    });

    let Some(scope) = native_scope else {
        // A `scope` wrapped in a Jinja block counts as present but stays unvalidated.
        if element
            .attrs
            .iter()
            .any(|attr| declares_native_attr(attr, "scope"))
        {
            return;
        }
        checker.report_diagnostic(
            &TableHeaderMissingScope {
                kind: ScopeViolation::MissingOrEmpty,
            },
            checker.source_span(element.tag_name),
        );
        return;
    };

    match scope.value {
        Some((value, _)) if !value.is_empty() => {
            if contains_interpolation(value)
                || VALID_SCOPE_VALUES
                    .iter()
                    .any(|valid| valid.eq_ignore_ascii_case(value))
            {
                return;
            }
            checker.report_diagnostic(
                &TableHeaderMissingScope {
                    kind: ScopeViolation::InvalidValue {
                        value: value.into(),
                    },
                },
                checker.source_span(value),
            );
        }
        // `scope=""` or a valueless `scope`: present but with no usable value.
        _ => {
            checker.report_diagnostic(
                &TableHeaderMissingScope {
                    kind: ScopeViolation::MissingOrEmpty,
                },
                checker.source_span(scope.name),
            );
        }
    }
}

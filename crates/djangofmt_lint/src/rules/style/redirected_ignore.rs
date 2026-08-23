use std::borrow::Cow;

use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};
use crate::{Checker, span};

/// ## What it does
/// Checks for djangofmt directives written in HTML comments, like `<!-- djangofmt:ignore -->`.
///
/// ## Why is this bad?
/// HTML comments are shipped to the client, so the directive ends up in every rendered page. And
/// only `{# #}` template comments carry suppression directives: `<!-- djangofmt: ignore[rule] -->`
/// silences nothing.
///
/// ## Example
/// ```html
/// <!-- djangofmt:ignore -->
/// <div   class="keep-this-unformatted"   >Content</div>
/// ```
///
/// Use instead:
/// ```html
/// {# djangofmt:ignore #}
/// <div   class="keep-this-unformatted"   >Content</div>
/// ```
///
/// ## Fix safety
/// Marked as safe: the formatter treats both comment styles the same, and the directive stops
/// leaking into rendered HTML.
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct RedirectedIgnore;

impl Violation for RedirectedIgnore {
    const RULE: Rule = Rule::RedirectedIgnore;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        "djangofmt directive in an HTML comment.".into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some("Move the directive into a `{# #}` template comment: it is stripped at render time, and it is the only style suppressions are read from.".into())
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Rewrite as a `{# #}` template comment")
    }
}

/// Lint one HTML comment; `body` is its inner text, `comment_raw` the whole `<!-- -->` slice.
pub fn check(body: &str, comment_raw: &str, checker: &Checker<'_>) {
    if !body.trim_start().starts_with("djangofmt:") {
        return;
    }
    let offset = checker.source_offset(comment_raw);
    let range = span(offset, comment_raw.len());
    let Some(mut guard) = checker.report_diagnostic_if_enabled(&RedirectedIgnore, range) else {
        return;
    };
    guard.set_fix(Fix::safe_edit(Edit::replacement(
        format!("{{#{body}#}}"),
        range,
    )));
}

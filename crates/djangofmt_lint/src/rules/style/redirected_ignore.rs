use std::borrow::Cow;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::suppression::{TEMPLATE_COMMENT_CLOSE, is_directive};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for djangofmt directives written in HTML comments, like `<!-- djangofmt:ignore -->`.
///
/// ## Why is this bad?
/// HTML comments are shipped to the client, so the directive ends up in every rendered page. And
/// only `{# #}` template comments carry suppressions: `<!-- djangofmt: ignore[rule] -->` silences
/// nothing. The formatter reads its own directive from either comment style, so moving it to a
/// `{# #}` comment changes nothing there.
///
/// No fix is offered for a comment spanning several lines, since Django's `{# #}` comments are
/// single-line, nor for one containing `#}`, which would end the new comment early.
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
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct RedirectedIgnore;

impl Violation for RedirectedIgnore {
    const RULE: Rule = Rule::RedirectedIgnore;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        "djangofmt directive in an HTML comment".into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some("Write it in a `{# #}` template comment instead".into())
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Rewrite as a `{# #}` template comment")
    }
}

/// Lint one HTML comment; `body` is its inner text, `comment_raw` the whole `<!-- -->` slice.
pub fn check(body: &str, comment_raw: &str, checker: &Checker<'_>) {
    if !is_directive(body) {
        return;
    }
    let range = checker.source_span(comment_raw);
    let mut guard = checker.report_diagnostic(&RedirectedIgnore, range);
    if !body.contains('\n') && !body.contains(TEMPLATE_COMMENT_CLOSE) {
        guard.set_fix(Fix::safe_edit(Edit::replacement(
            format!("{{#{body}#}}"),
            range,
        )));
    }
}

use std::borrow::Cow;

use markup_fmt::ast::Comment;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::helpers::{
    HTML_COMMENT_CLOSE, HTML_COMMENT_OPEN, TEMPLATE_COMMENT_CLOSE, enclosing_comment,
};
use crate::registry::{Rule, RuleCategory};
use crate::suppression::IgnoreDirective;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for ignore comments written as HTML comments, like `<!-- djangofmt:ignore -->`.
///
/// ## Why is this bad?
/// HTML ignore comments are deprecated: an HTML comment is shipped to the client, so the
/// directive ends up in every rendered page, and only `{# #}` template comments carry lint
/// suppressions, so `<!-- djangofmt: ignore[rule] -->` silences nothing. This rule migrates them
/// to `{# #}` template comments, which the formatter honors the same way.
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
        "Deprecated HTML ignore comment".into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some("Write it as a `{# #}` template comment instead".into())
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Rewrite as a `{# #}` template comment")
    }
}

pub fn check(comment: &Comment<'_>, checker: &Checker<'_>) {
    let body = comment.raw;
    if !IgnoreDirective::is_addressed(body) {
        return;
    }
    let raw = enclosing_comment(checker, body, HTML_COMMENT_OPEN, HTML_COMMENT_CLOSE);
    let range = checker.source_span(raw);
    let mut guard = checker.report_diagnostic(&RedirectedIgnore, range);
    if !body.contains('\n') && !body.contains(TEMPLATE_COMMENT_CLOSE) {
        guard.set_fix(Fix::safe_edit(Edit::replacement(
            format!("{{#{body}#}}"),
            range,
        )));
    }
}

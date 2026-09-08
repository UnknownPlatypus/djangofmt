use std::borrow::Cow;

use markup_fmt::ast::Comment;

use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::{HTML_COMMENT, TEMPLATE_COMMENT};
use crate::suppression::IGNORE_DIRECTIVE;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};
use crate::{Checker, span};

/// ## What it does
/// Checks for the formatter's `djangofmt:ignore` directive written as an HTML comment.
///
/// ## Why is this bad?
/// `<!-- djangofmt:ignore -->` is the deprecated spelling of `{# djangofmt:ignore #}`. An HTML
/// comment is shipped to the client, so the directive ends up in every rendered page, whereas the
/// template engine drops a `{# #}` comment.
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
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;
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
    if !markup_fmt::matches_directive(body, IGNORE_DIRECTIVE) {
        return;
    }
    let range = HTML_COMMENT.enclosing_range(checker, body);
    let span = span(range.start, range.len());
    let mut guard = checker.report_diagnostic(&RedirectedIgnore, span);
    // Django `{# #}` comments are single-line, and a `#}` in the body would end the new one early.
    if !body.contains('\n') && !body.contains(TEMPLATE_COMMENT.close) {
        guard.set_fix(Fix::safe_edit(Edit::replacement(
            format!("{{#{body}#}}"),
            span,
        )));
    }
}

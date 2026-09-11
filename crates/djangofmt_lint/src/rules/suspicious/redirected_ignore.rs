use std::borrow::Cow;

use markup_fmt::ast::Comment;

use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::{HTML_COMMENT, TEMPLATE_COMMENT};
use crate::suppression::FORMAT_IGNORE_DIRECTIVES;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};
use crate::{Checker, span};

/// ## What it does
/// Checks for the formatter's ignore directive written as an HTML comment.
///
/// ## Why is this bad?
/// `<!-- djangofmt:ignore -->` is the deprecated spelling of `{# djangofmt:ignore #}`. The
/// formatter reads its directive from HTML comments too, but where the template engine drops a
/// `{# #}` comment, an HTML comment is shipped to the client, so the directive ends up in every
/// rendered page.
///
/// The fix rewrites the comment in place, free-text reason included. A comment spanning several
/// lines is reported but not rewritten: Django's `{# #}` comments are single-line.
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
pub struct RedirectedIgnore {
    /// Why the rewrite is left to the author, when it is.
    pub unfixable: Option<Unfixable>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unfixable {
    /// Django's `{# #}` comments are single-line.
    MultiLine,
    /// A `#}` in the body would end the new comment early.
    ClosesEarly,
}

impl Violation for RedirectedIgnore {
    const RULE: Rule = Rule::RedirectedIgnore;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        "Deprecated HTML ignore comment".into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some(match self.unfixable {
            None => "Write it as a `{# #}` template comment instead".into(),
            Some(Unfixable::MultiLine) => {
                "Write it as a single-line `{# #}` template comment".into()
            }
            Some(Unfixable::ClosesEarly) => {
                "Write it as a `{# #}` template comment, without the `#}` in its body".into()
            }
        })
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Rewrite as a `{# #}` template comment")
    }
}

pub fn check(comment: &Comment<'_>, checker: &Checker<'_>) {
    let body = comment.raw;
    if !FORMAT_IGNORE_DIRECTIVES
        .iter()
        .any(|directive| markup_fmt::matches_directive(body, directive))
    {
        return;
    }
    let range = HTML_COMMENT.enclosing_range(checker, body);
    let span = span(range.start, range.len());
    let unfixable = if body.contains('\n') {
        Some(Unfixable::MultiLine)
    } else if body.contains(TEMPLATE_COMMENT.close) {
        Some(Unfixable::ClosesEarly)
    } else {
        None
    };
    let mut guard = checker.report_diagnostic(&RedirectedIgnore { unfixable }, span);
    if unfixable.is_none() {
        // Edge dashes of a `<!--- --->` comment would read as `{#-`/`-#}` whitespace control.
        let body = body.trim_matches(|c: char| c.is_whitespace() || c == '-');
        guard.set_fix(Fix::safe_edit(Edit::replacement(
            format!("{{# {body} #}}"),
            span,
        )));
    }
}

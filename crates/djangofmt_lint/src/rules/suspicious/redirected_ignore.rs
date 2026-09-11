use std::borrow::Cow;

use markup_fmt::ast::Comment;

use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::{HTML_COMMENT, TEMPLATE_COMMENT, strip_bom};
use crate::suppression::{FORMAT_IGNORE_DIRECTIVES, IGNORE_DIRECTIVE, canonical_ignore_comment};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};
use crate::{Checker, span};

/// ## What it does
/// Checks for the formatter's ignore directive written as an HTML comment.
///
/// ## Why is this bad?
/// `<!-- djangofmt:ignore -->` is the deprecated spelling of `{# djangofmt: ignore[format] #}`.
/// The formatter reads its directive from HTML comments too, but where the template engine drops
/// a `{# #}` comment, an HTML comment is shipped to the client, so the directive ends up in every
/// rendered page.
///
/// The fix spells the directive out, free-text reason included: `ignore[format]` before a node,
/// `file-ignore[format]` when the comment leads the file. A comment spanning several lines is
/// reported but not rewritten: Django's `{# #}` comments are single-line.
///
/// ## Example
/// ```html
/// <!-- djangofmt:ignore -->
/// <div   class="keep-this-unformatted"   >Content</div>
/// ```
///
/// Use instead:
/// ```html
/// {# djangofmt: ignore[format] #}
/// <div   class="keep-this-unformatted"   >Content</div>
/// ```
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct RedirectedIgnore {
    /// Whether the comment is the legacy whole-file opt-out, spelled `file-ignore[format]`.
    pub file_level: bool,
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
        let spelling = if self.file_level {
            "{# djangofmt: file-ignore[format] #}"
        } else {
            "{# djangofmt: ignore[format] #}"
        };
        Some(
            match self.unfixable {
                None => format!("Write it as `{spelling}` instead"),
                Some(Unfixable::MultiLine) => format!("Write it as `{spelling}` on a single line"),
                Some(Unfixable::ClosesEarly) => {
                    format!("Write it as `{spelling}`, without the `#}}` in its body")
                }
            }
            .into(),
        )
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some(if self.file_level {
            "Rewrite as `{# djangofmt: file-ignore[format] #}`"
        } else {
            "Rewrite as `{# djangofmt: ignore[format] #}`"
        })
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
    // Leading the file, the bare directive is the legacy whole-file opt-out.
    let file_level = strip_bom(&checker.context().source()[..range.start]).is_empty()
        && markup_fmt::matches_directive(body, IGNORE_DIRECTIVE);
    let unfixable = if body.contains('\n') {
        Some(Unfixable::MultiLine)
    } else if body.contains(TEMPLATE_COMMENT.close) {
        Some(Unfixable::ClosesEarly)
    } else {
        None
    };
    let violation = RedirectedIgnore {
        file_level,
        unfixable,
    };
    let mut guard = checker.report_diagnostic(&violation, span);
    if unfixable.is_none()
        && let Some(comment) = canonical_ignore_comment(body, file_level)
    {
        guard.set_fix(Fix::safe_edit(Edit::replacement(comment, span)));
    }
}

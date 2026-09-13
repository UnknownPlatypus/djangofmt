use std::borrow::Cow;

use markup_fmt::ast::Comment;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::{HTML_COMMENT, TEMPLATE_COMMENT, strip_bom};
use crate::suppression::FormatIgnoreDirective;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for the formatter's ignore directive written as an HTML comment.
///
/// ## Why is this bad?
/// `<!-- djangofmt:ignore -->` is the deprecated spelling of `{# djangofmt: ignore[format] #}`.
/// Prefer the django comment form because the HTML comment is shipped to the client,
/// so the directive ends up in every rendered page.
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
///
/// ## Fix safety
/// The fix is marked as unsafe when the comment lists a code beyond `format`: the linter reads
/// no HTML comment, so that code silences nothing today, and the `{# #}` rewrite would start
/// honoring it.
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

impl Unfixable {
    /// Why the comment body cannot move into a `{# #}` comment as is, if it cannot.
    fn in_body(body: &str) -> Option<Self> {
        if body.contains('\n') {
            Some(Self::MultiLine)
        } else if body.contains(TEMPLATE_COMMENT.close) {
            Some(Self::ClosesEarly)
        } else {
            None
        }
    }
}

impl RedirectedIgnore {
    /// The `{# #}` comment the directive should be written as.
    const fn spelling(&self) -> &'static str {
        if self.file_level {
            "{# djangofmt: file-ignore[format] #}"
        } else {
            "{# djangofmt: ignore[format] #}"
        }
    }
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
        let spelling = self.spelling();
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
    let Some(directive) = FormatIgnoreDirective::parse(comment.raw) else {
        return;
    };
    let html_comment = HTML_COMMENT.enclosing_comment(checker, comment.raw);
    let violation = RedirectedIgnore {
        file_level: is_legacy_file_opt_out(&directive, html_comment, checker),
        unfixable: Unfixable::in_body(comment.raw),
    };
    let span = checker.source_span(html_comment);
    let mut guard = checker.report_diagnostic(&violation, span);
    if violation.unfixable.is_none() {
        let rewritten = directive.to_template_comment(violation.file_level);
        let edit = Edit::replacement(rewritten, span);
        guard.set_fix(if directive.is_format_only() {
            Fix::safe_edit(edit)
        } else {
            Fix::unsafe_edit(edit)
        });
    }
}

/// Leading the file, the formatter's bare directive is its legacy whole-file opt-out.
/// Anything before it but a BOM, whitespace included, makes it guard the first node instead.
fn is_legacy_file_opt_out(
    directive: &FormatIgnoreDirective<'_>,
    html_comment: &str,
    checker: &Checker<'_>,
) -> bool {
    let before = &checker.context().source()[..checker.source_offset(html_comment)];
    directive.is_bare() && strip_bom(before).is_empty()
}

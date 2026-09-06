use std::borrow::Cow;

use crate::fix::edits::{delete_codes_or_comment, delete_comment};
use crate::fix::{Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::suppression::{Directive, DirectiveComment, INVALID_SYNTAX_CODE, ParseErrorKind};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};
use crate::{Checker, span};

#[derive(Debug, PartialEq, Eq)]
pub enum IgnoreCommentViolation {
    /// The comment starts with `djangofmt:` but does not parse as a directive.
    Malformed(ParseErrorKind),
    /// A `file-ignore[...]` directive that does not lead the file.
    MisplacedFileIgnore,
    /// `invalid-syntax` listed in a node-level `ignore[...]`.
    InvalidSyntaxOnNode,
}

/// ## What it does
/// Checks for `djangofmt:` suppression comments that are malformed or misplaced.
///
/// ## Why is this bad?
/// Invalid suppression comments are ignored by djangofmt, and should either be fixed or removed to avoid confusion.
///
/// ## Example
/// ```html
/// {# djangofmt: ignore[] #}
/// <form method="yes">Submit</form>
/// ```
///
/// Use instead:
/// ```html
/// {# djangofmt: ignore[invalid-attr-value] #}
/// <form method="yes">Submit</form>
/// ```
///
/// Or delete the invalid suppression comment.
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct InvalidIgnoreComment {
    pub kind: IgnoreCommentViolation,
}

impl Violation for InvalidIgnoreComment {
    const RULE: Rule = Rule::InvalidIgnoreComment;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        match self.kind {
            IgnoreCommentViolation::Malformed(error) => {
                format!("Invalid suppression comment: {error}").into()
            }
            IgnoreCommentViolation::MisplacedFileIgnore => {
                "Invalid suppression comment: file-level suppressions must be at the top of the file"
                    .into()
            }
            IgnoreCommentViolation::InvalidSyntaxOnNode => {
                "Invalid suppression comment: `invalid-syntax` cannot be scoped to a node".into()
            }
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some(match self.kind {
            IgnoreCommentViolation::Malformed(_) => {
                "Use `{# djangofmt: ignore[rule, ...] #}` or `{# djangofmt: file-ignore[rule, ...] #}`"
                    .into()
            }
            IgnoreCommentViolation::MisplacedFileIgnore => {
                "Move the comment to the top of the file, or use `ignore[...]`".into()
            }
            IgnoreCommentViolation::InvalidSyntaxOnNode => {
                "Use `{# djangofmt: file-ignore[invalid-syntax] #}` at the top of the file instead"
                    .into()
            }
        })
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some(match self.kind {
            IgnoreCommentViolation::InvalidSyntaxOnNode => "Remove `invalid-syntax` code",
            IgnoreCommentViolation::Malformed(_) | IgnoreCommentViolation::MisplacedFileIgnore => {
                "Remove suppression comment"
            }
        })
    }
}

/// Lint every directive comment of the file.
pub fn check(directives: &[DirectiveComment<'_>], checker: &Checker<'_>) {
    for comment in directives {
        check_comment(comment, checker);
    }
}

fn check_comment(comment: &DirectiveComment<'_>, checker: &Checker<'_>) {
    let ctx = checker.context();
    let remove_comment = |kind| {
        let span = span(ctx.source_offset(comment.raw), comment.raw.len());
        (
            kind,
            span,
            Fix::unsafe_edit(delete_comment(ctx, comment.raw)),
        )
    };
    let (kind, span, fix) = match &comment.directive {
        Directive::Malformed(error) => remove_comment(IgnoreCommentViolation::Malformed(*error)),
        Directive::FileIgnore(_) if !comment.is_leading => {
            remove_comment(IgnoreCommentViolation::MisplacedFileIgnore)
        }
        Directive::FileIgnore(_) => return,
        Directive::Ignore(codes) => {
            if !codes.contains(&INVALID_SYNTAX_CODE) {
                return;
            }
            let (span, edit) =
                delete_codes_or_comment(ctx, comment.raw, codes, &[INVALID_SYNTAX_CODE]);
            (
                IgnoreCommentViolation::InvalidSyntaxOnNode,
                span,
                Fix::safe_edit(edit),
            )
        }
    };
    let mut guard = checker.report_diagnostic(&InvalidIgnoreComment { kind }, span);
    guard.set_fix(fix);
}

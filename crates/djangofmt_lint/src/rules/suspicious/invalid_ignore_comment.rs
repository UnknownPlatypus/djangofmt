use std::borrow::Cow;

use crate::registry::{Rule, RuleCategory};
use crate::suppression::{Directive, FORMAT, INVALID_SYNTAX};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};
use crate::{Checker, span};

#[derive(Debug, PartialEq, Eq)]
pub enum IgnoreCommentViolation {
    /// The comment starts with `djangofmt:` but is not a recognized directive.
    Malformed,
    /// A `file-ignore[...]` directive that is not the file's first comment.
    MisplacedFileIgnore,
    /// `invalid-syntax` listed in a node-level `ignore[...]`.
    InvalidSyntaxOnNode,
    /// `format` listed in a node-level `ignore[...]`.
    FormatOnNode,
}

/// ## What it does
/// Checks for `djangofmt:` suppression comments that are malformed or misplaced.
///
/// ## Why is this bad?
/// A directive djangofmt does not recognize is skipped silently, so the diagnostics it was meant to
/// silence keep firing while the author believes they are suppressed.
///
/// This covers a syntax error in the directive (missing bracket, empty rule list, trailing text), a
/// `file-ignore[...]` that is not the file's first comment, and the file-wide codes `invalid-syntax`
/// and `format` listed in a node-level `ignore[...]`.
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
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct InvalidIgnoreComment {
    pub kind: IgnoreCommentViolation,
}

impl Violation for InvalidIgnoreComment {
    const RULE: Rule = Rule::InvalidIgnoreComment;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        match self.kind {
            IgnoreCommentViolation::Malformed => "Malformed `djangofmt:` directive comment.".into(),
            IgnoreCommentViolation::MisplacedFileIgnore => {
                "`file-ignore[...]` must be the first comment in the file.".into()
            }
            IgnoreCommentViolation::InvalidSyntaxOnNode => {
                "`invalid-syntax` cannot be scoped to a node.".into()
            }
            IgnoreCommentViolation::FormatOnNode => "`format` cannot be scoped to a node.".into(),
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some(match self.kind {
            IgnoreCommentViolation::Malformed => {
                "Write `{# djangofmt: ignore[rule] #}` before a node, or `{# djangofmt: file-ignore[rule] #}` at the top of the file.".into()
            }
            IgnoreCommentViolation::MisplacedFileIgnore => {
                "Move the comment to the top of the file, or use `ignore[...]` to target the next node.".into()
            }
            IgnoreCommentViolation::InvalidSyntaxOnNode => {
                "An unparsable file has no nodes to attach to: use `{# djangofmt: file-ignore[invalid-syntax] #}` at the top of the file.".into()
            }
            IgnoreCommentViolation::FormatOnNode => {
                "Use a bare `{# djangofmt:ignore #}` comment to skip formatting the next node.".into()
            }
        })
    }
}

/// Lint one directive comment; `directive` is its parse outcome.
pub fn check(
    directive: Option<&Directive<'_>>,
    comment_raw: &str,
    is_file_head: bool,
    checker: &Checker<'_>,
) {
    // Point at the offending code when one is singled out, at the whole comment otherwise.
    let (kind, slice) = match directive {
        None => (IgnoreCommentViolation::Malformed, comment_raw),
        Some(Directive::FileIgnore(_)) if !is_file_head => {
            (IgnoreCommentViolation::MisplacedFileIgnore, comment_raw)
        }
        Some(Directive::Ignore(codes)) => {
            if let Some(code) = codes.iter().find(|code| **code == INVALID_SYNTAX) {
                (IgnoreCommentViolation::InvalidSyntaxOnNode, *code)
            } else if let Some(code) = codes.iter().find(|code| **code == FORMAT) {
                (IgnoreCommentViolation::FormatOnNode, *code)
            } else {
                return;
            }
        }
        Some(Directive::FileIgnore(_)) => return,
    };
    let offset = checker.source_offset(slice);
    checker.report_diagnostic_if_enabled(&InvalidIgnoreComment { kind }, span(offset, slice.len()));
}

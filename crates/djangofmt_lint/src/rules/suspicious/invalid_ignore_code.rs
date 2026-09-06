use std::borrow::Cow;
use std::str::FromStr;

use crate::Checker;
use crate::fix::edits::delete_codes_or_comment;
use crate::fix::{Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::suppression::{IgnoreComment, ReservedCode};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `ignore[...]` / `file-ignore[...]` suppression comments listing an invalid code.
///
/// ## Why is this bad?
/// An unknown code suppresses nothing, so the diagnostic the comment was meant to silence keeps firing.
/// It is usually a typo, or a leftover from a rule that was renamed.
///
/// ## Example
/// ```html
/// {# djangofmt: ignore[not-a-rule] #}
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
pub struct InvalidIgnoreCode {
    /// The invalid codes, comma-separated.
    pub codes: String,
    /// Whether every listed code is invalid, so the fix removes the whole comment.
    pub whole_comment: bool,
}

impl Violation for InvalidIgnoreCode {
    const RULE: Rule = Rule::InvalidIgnoreCode;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        format!("Invalid rule code in suppression: {}", self.codes).into()
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some(if self.whole_comment {
            "Remove suppression comment"
        } else {
            "Remove invalid rule code"
        })
    }
}

/// Whether `code` names a rule or one of the reserved codes.
fn is_known(code: &str) -> bool {
    Rule::from_str(code).is_ok() || ReservedCode::from_str(code).is_ok()
}

/// Report the codes naming neither a rule nor a reserved code, once per comment.
pub fn check(comments: &[IgnoreComment<'_>], checker: &Checker<'_>) {
    for comment in comments {
        check_comment(comment, checker);
    }
}

fn check_comment(comment: &IgnoreComment<'_>, checker: &Checker<'_>) {
    let codes = comment.directive.codes();
    let invalid: Vec<&str> = codes
        .iter()
        .copied()
        .filter(|code| !is_known(code))
        .collect();
    if invalid.is_empty() {
        return;
    }
    let (span, edit) = delete_codes_or_comment(checker.context(), comment.raw, codes, &invalid);
    let violation = InvalidIgnoreCode {
        codes: invalid
            .iter()
            .map(|code| format!("`{code}`"))
            .collect::<Vec<_>>()
            .join(", "),
        whole_comment: invalid.len() == codes.len(),
    };
    let mut guard = checker.report_diagnostic(&violation, span);
    guard.set_fix(Fix::safe_edit(edit));
}

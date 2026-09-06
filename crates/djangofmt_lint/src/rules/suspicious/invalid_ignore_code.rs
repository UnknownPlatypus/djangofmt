use std::borrow::Cow;
use std::str::FromStr;

use strum::{IntoEnumIterator, VariantNames};

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
///
/// ## Fix safety
/// The fix is marked as unsafe when every listed code is invalid, because the whole comment is
/// then deleted, taking any free-text reason with it. Dropping an invalid code from a list that
/// keeps a valid one is safe.
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct InvalidIgnoreCode {
    /// The invalid codes, comma-separated.
    pub codes: String,
    /// The known code a lone invalid one is close enough to be a typo for.
    pub suggestion: Option<&'static str>,
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

    fn help(&self) -> Option<Cow<'static, str>> {
        self.suggestion
            .map(|code| format!("Did you mean `{code}`?").into())
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some(if self.whole_comment {
            "Remove suppression comment"
        } else {
            "Remove invalid rule code"
        })
    }
}

/// Report the codes naming neither a rule nor a reserved code, once per comment.
pub fn check(comments: &[IgnoreComment<'_>], checker: &Checker<'_>) {
    for comment in comments {
        check_comment(comment, checker);
    }
}

fn check_comment(comment: &IgnoreComment<'_>, checker: &Checker<'_>) {
    let codes = comment.directive.codes();
    let mut invalid: Vec<&str> = Vec::new();
    for &code in codes {
        let unknown = Rule::from_str(code).is_err() && ReservedCode::from_str(code).is_err();
        // The same code twice is one mistake, and the fix drops every occurrence of it.
        if unknown && !invalid.contains(&code) {
            invalid.push(code);
        }
    }
    if invalid.is_empty() {
        return;
    }
    let deletion = delete_codes_or_comment(checker.context(), comment.raw, codes, &invalid);
    let violation = InvalidIgnoreCode {
        codes: invalid
            .iter()
            .map(|code| format!("`{code}`"))
            .collect::<Vec<_>>()
            .join(", "),
        // With several invalid codes the diagnostic spans the comment, so a lone suggestion
        // would not say which code it is about.
        suggestion: match invalid.as_slice() {
            [code] => closest_known_code(code),
            _ => None,
        },
        whole_comment: deletion.whole_comment,
    };
    let mut guard = checker.report_diagnostic(&violation, deletion.span);
    guard.set_fix(if deletion.whole_comment {
        Fix::unsafe_edit(deletion.edit)
    } else {
        Fix::safe_edit(deletion.edit)
    });
}

/// The known code `code` was likely meant to be, when one is close enough to name.
fn closest_known_code(code: &str) -> Option<&'static str> {
    Rule::iter()
        .map(<&'static str>::from)
        .chain(ReservedCode::VARIANTS.iter().copied())
        .map(|known| (strsim::levenshtein(code, known), known))
        // A third of the longer spelling, so a suggestion stays a plausible misspelling.
        .filter(|&(distance, known)| distance <= (code.len().max(known.len()) / 3).max(1))
        .min_by_key(|&(distance, _)| distance)
        .map(|(_, known)| known)
}

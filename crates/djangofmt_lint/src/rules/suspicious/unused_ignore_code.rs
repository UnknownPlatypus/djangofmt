use std::borrow::Cow;
use std::str::FromStr;

use crate::Checker;
use crate::fix::edits::delete_codes_or_comment;
use crate::fix::{Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::suppression::{IgnoreComment, IgnoreDirective, ReservedCode};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `ignore[...]` / `file-ignore[...]` suppression comments listing a code that silences
/// nothing: the rule reported no diagnostic where the comment applies, the rule is not enabled, or
/// the code is already listed in the same comment.
///
/// ## Why is this bad?
/// A suppression that no longer matches any diagnostic is likely a leftover from markup that was
/// since fixed, and should be removed to avoid confusion. Left in place, it would also hide a new
/// violation of that rule.
///
/// Codes naming no rule are left to `invalid-ignore-code`. `format` is never reported, as whether
/// the formatter needs it is not the linter's to know. `invalid-syntax` is reported once the file
/// parses again.
///
/// ## Example
/// ```html
/// {# djangofmt: ignore[invalid-attr-value, empty-attr-value] #}
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
/// The fix is marked as unsafe when every listed code is unused, because the whole comment is then
/// deleted, taking any free-text reason with it. Dropping an unused code from a list that keeps a
/// used one is safe.
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct UnusedIgnoreCode {
    /// Codes whose rule reported nothing where the comment applies, comma-separated.
    pub unmatched: String,
    /// Codes whose rule is not enabled, comma-separated.
    pub disabled: String,
    /// Codes repeating an earlier one of the same comment, comma-separated.
    pub duplicated: String,
    /// Whether every listed code is unused, so the fix removes the whole comment.
    pub whole_comment: bool,
}

impl UnusedIgnoreCode {
    /// The unused codes grouped by reason; a bare group is the unmatched one.
    fn reasons(&self) -> String {
        let mut reasons = Vec::new();
        if !self.unmatched.is_empty() {
            reasons.push(self.unmatched.clone());
        }
        if !self.disabled.is_empty() {
            reasons.push(format!("non-enabled: {}", self.disabled));
        }
        if !self.duplicated.is_empty() {
            reasons.push(format!("duplicated: {}", self.duplicated));
        }
        reasons.join("; ")
    }
}

impl Violation for UnusedIgnoreCode {
    const RULE: Rule = Rule::UnusedIgnoreCode;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        format!("Unused rule code in suppression: {}", self.reasons()).into()
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some(if self.whole_comment {
            "Remove suppression comment"
        } else {
            "Remove unused rule code"
        })
    }
}

/// Why a listed code silences nothing.
#[derive(Clone, Copy)]
enum Unused {
    /// The rule is enabled but reported nothing where the comment applies.
    Unmatched,
    /// The rule is not enabled, so it could not have reported anything.
    Disabled,
    /// An earlier code of the same comment already covers it.
    Duplicated,
}

/// Report the unused codes of every comment, once per comment.
pub fn check(comments: &[IgnoreComment<'_>], checker: &Checker<'_>) {
    let own_code: &str = Rule::UnusedIgnoreCode.into();
    // The rule runs after suppression, so its own file-level opt-out is honored here.
    let file_ignored = comments.iter().any(|comment| {
        comment.is_leading
            && matches!(&comment.directive, IgnoreDirective::FileIgnore(codes) if codes.contains(&own_code))
    });
    if file_ignored {
        return;
    }
    for comment in comments {
        check_comment(comment, own_code, checker);
    }
}

fn check_comment(comment: &IgnoreComment<'_>, own_code: &str, checker: &Checker<'_>) {
    let (codes, is_file_level) = match &comment.directive {
        IgnoreDirective::Ignore(codes) => (codes.as_slice(), false),
        IgnoreDirective::FileIgnore(codes) if comment.is_leading => (codes.as_slice(), true),
        // Malformed and misplaced directives are `invalid-ignore-comment`'s to report.
        IgnoreDirective::FileIgnore(_) | IgnoreDirective::Malformed(_) => return,
    };
    // A comment silencing this very rule is left alone, whatever else it lists.
    if codes.contains(&own_code) {
        return;
    }

    let mut remove = Vec::new();
    let mut by_reason: [Vec<&str>; 3] = Default::default();
    for (index, &code) in codes.iter().enumerate() {
        let Some(reason) = classify(code, &codes[..index], comment, is_file_level, checker) else {
            continue;
        };
        remove.push(index);
        by_reason[reason as usize].push(code);
    }
    if remove.is_empty() {
        return;
    }

    let deletion = delete_codes_or_comment(checker.context(), comment.raw, codes, &remove);
    let [unmatched, disabled, duplicated] = by_reason.map(|codes| quote_list(&codes));
    let violation = UnusedIgnoreCode {
        unmatched,
        disabled,
        duplicated,
        whole_comment: deletion.whole_comment,
    };
    let mut guard = checker.report_diagnostic(&violation, deletion.span);
    guard.set_fix(if deletion.whole_comment {
        Fix::unsafe_edit(deletion.edit)
    } else {
        Fix::safe_edit(deletion.edit)
    });
}

/// Why `code` is unused, `None` when it is used or not this rule's to judge.
fn classify(
    code: &str,
    earlier: &[&str],
    comment: &IgnoreComment<'_>,
    is_file_level: bool,
    checker: &Checker<'_>,
) -> Option<Unused> {
    let rule = Rule::from_str(code).ok();
    let reserved = ReservedCode::from_str(code).ok();
    // An unknown code is `invalid-ignore-code`'s, repeated or not.
    if rule.is_none() && reserved.is_none() {
        return None;
    }
    if earlier.contains(&code) {
        return Some(Unused::Duplicated);
    }
    match rule {
        Some(rule) if comment.matched.contains(rule) => None,
        Some(rule) if checker.is_rule_enabled(rule) => Some(Unused::Unmatched),
        Some(_) => Some(Unused::Disabled),
        // The file parsed, so there is no syntax error left to suppress.
        // On a node the code is misplaced, which `invalid-ignore-comment` reports.
        None if is_file_level && reserved == Some(ReservedCode::InvalidSyntax) => {
            Some(Unused::Unmatched)
        }
        // `format` speaks to the formatter, which the linter cannot see.
        None => None,
    }
}

fn quote_list(codes: &[&str]) -> String {
    codes
        .iter()
        .map(|code| format!("`{code}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

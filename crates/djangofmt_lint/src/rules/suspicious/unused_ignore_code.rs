use std::borrow::Cow;
use std::str::FromStr;

use crate::Checker;
use crate::fix::FixAvailability;
use crate::fix::edits::delete_codes_or_comment;
use crate::registry::{Rule, RuleCategory};
use crate::rule_set::RuleSet;
use crate::suppression::{IgnoreComment, IgnoreScope, ReservedCode};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `ignore[...]` / `file-ignore[...]` suppression comments listing a rule code that
/// silences nothing.
///
/// ## Why is this bad?
/// A suppression matching no diagnostic is usually a leftover from markup that was since fixed.
/// It adds noise, and goes on hiding the next real violation of that rule at the same spot.
///
/// A code naming no rule at all is left to `invalid-ignore-code`. `format` addresses the
/// formatter, which the linter cannot see, so it counts as used unless the same comment already
/// lists it. `invalid-syntax` counts as used only while the file fails to parse.
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
    /// The unused codes, grouped by reason: `` `a`; `b`, `c` (non-enabled) ``.
    pub codes: String,
    /// Whether every listed code is unused, so the fix removes the whole comment.
    pub whole_comment: bool,
}

impl Violation for UnusedIgnoreCode {
    const RULE: Rule = Rule::UnusedIgnoreCode;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        format!("Unused rule code in suppression: {}", self.codes).into()
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
#[derive(Clone, Copy, PartialEq, Eq)]
enum Unused {
    /// The rule is enabled but reported nothing where the comment applies.
    Unmatched,
    /// The rule is not enabled, so it could not have reported anything.
    Disabled,
    /// An earlier code of the same comment already covers it.
    Duplicated,
}

impl Unused {
    /// In message order: the unmatched codes lead, unlabeled, as the common case.
    const ALL: [Self; 3] = [Self::Unmatched, Self::Disabled, Self::Duplicated];

    const fn label(self) -> &'static str {
        match self {
            Self::Unmatched => "",
            Self::Disabled => " (non-enabled)",
            Self::Duplicated => " (duplicated)",
        }
    }
}

/// Report the unused codes of every comment, once per comment.
pub fn check(comments: &[IgnoreComment<'_>], checker: &Checker<'_>) {
    let own_code: &str = Rule::UnusedIgnoreCode.into();
    // The rule runs after suppression, so its own file-level opt-out is honored here.
    let file_ignored = comments.iter().any(|comment| {
        matches!(comment.in_force(), Some((codes, IgnoreScope::File)) if codes.contains(&own_code))
    });
    if file_ignored {
        return;
    }
    for comment in comments {
        check_comment(comment, own_code, checker);
    }
}

fn check_comment(comment: &IgnoreComment<'_>, own_code: &str, checker: &Checker<'_>) {
    // Malformed and misplaced directives are `invalid-ignore-comment`'s to report.
    let Some((codes, scope)) = comment.in_force() else {
        return;
    };
    // A comment silencing this very rule is left alone, whatever else it lists.
    if codes.contains(&own_code) {
        return;
    }

    let mut remove = Vec::new();
    let mut unused = Vec::new();
    for (index, &code) in codes.iter().enumerate() {
        if let Some(reason) = classify(code, &codes[..index], comment.matched, scope, checker) {
            remove.push(index);
            unused.push((reason, code));
        }
    }
    if remove.is_empty() {
        return;
    }

    let deletion = delete_codes_or_comment(checker.context(), comment.raw, codes, &remove);
    let violation = UnusedIgnoreCode {
        codes: format_by_reason(&unused),
        whole_comment: deletion.whole_comment,
    };
    deletion.report(checker.context(), &violation);
}

/// Why `code` is unused, `None` when it is used or not this rule's to judge.
///
/// `matched` holds the rules the comment silenced, `earlier` the codes it lists before `code`.
fn classify(
    code: &str,
    earlier: &[&str],
    matched: RuleSet,
    scope: IgnoreScope,
    checker: &Checker<'_>,
) -> Option<Unused> {
    let rule = Rule::from_str(code);
    let reserved = ReservedCode::from_str(code);
    // A code naming no rule is `invalid-ignore-code`'s and `invalid-syntax` on a node is
    // `invalid-ignore-comment`'s, repeated or not.
    let theirs = match (rule, reserved) {
        (Err(_), Err(_)) => true,
        (Err(_), Ok(ReservedCode::InvalidSyntax)) => scope == IgnoreScope::Node,
        _ => false,
    };
    if theirs {
        return None;
    }
    if earlier.contains(&code) {
        return Some(Unused::Duplicated);
    }
    match (rule, reserved) {
        (Ok(rule), _) if matched.contains(rule) => None,
        (Ok(rule), _) if checker.is_rule_enabled(rule) => Some(Unused::Unmatched),
        (Ok(_), _) => Some(Unused::Disabled),
        // The file parsed, so there is no syntax error left to suppress.
        (Err(_), Ok(ReservedCode::InvalidSyntax)) => Some(Unused::Unmatched),
        // `format` speaks to the formatter, which the linter cannot see.
        (Err(_), _) => None,
    }
}

/// The unused codes grouped by reason, `; ` between groups: `` `a`; `b`, `c` (non-enabled) ``.
fn format_by_reason(unused: &[(Unused, &str)]) -> String {
    Unused::ALL
        .iter()
        .filter_map(|&reason| {
            let codes = unused
                .iter()
                .filter(|(cause, _)| *cause == reason)
                .map(|(_, code)| format!("`{code}`"))
                .collect::<Vec<_>>();
            (!codes.is_empty()).then(|| format!("{}{}", codes.join(", "), reason.label()))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

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
/// A code is unused when its rule reported nothing where the comment applies, when the rule is
/// not enabled, or when the same comment already lists it. `format` addresses the formatter, so
/// it is only ever unused as a repeat; `invalid-syntax` is unused once the file parses again.
/// A code naming no rule is `invalid-ignore-code`'s to report, and a node-level `invalid-syntax`
/// is `invalid-ignore-comment`'s.
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
/// The fix is marked as unsafe when it deletes a comment along with the free-text reason after
/// its code list, as in `ignore[...]: reason`. Dropping a code from a list, or a comment carrying
/// no reason, is safe.
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
pub fn check(checker: &Checker<'_>, comments: &[IgnoreComment<'_>]) {
    let own_code: &str = Rule::UnusedIgnoreCode.into();
    // The rule runs after suppression, so its own file-level opt-out is honored here.
    let file_ignored = comments.iter().any(|comment| {
        matches!(comment.in_force(), Some((codes, IgnoreScope::File)) if codes.contains(&own_code))
    });
    if file_ignored {
        return;
    }
    for comment in comments {
        check_comment(checker, comment, own_code);
    }
}

fn check_comment(checker: &Checker<'_>, comment: &IgnoreComment<'_>, own_code: &str) {
    // Malformed and misplaced directives are `invalid-ignore-comment`'s to report.
    let Some((codes, scope)) = comment.in_force() else {
        return;
    };
    // A comment silencing this very rule is left alone, whatever else it lists.
    if codes.contains(&own_code) {
        return;
    }

    // Indexed rather than by code, so of a repeated code only the repeat is dropped.
    let unused: Vec<(usize, Unused)> = codes
        .iter()
        .enumerate()
        .filter_map(|(index, &code)| {
            classify(checker, code, &codes[..index], comment.matched, scope)
                .map(|reason| (index, reason))
        })
        .collect();
    if unused.is_empty() {
        return;
    }

    let deletion = delete_codes_or_comment(checker.context(), comment, |index, _| {
        unused
            .iter()
            .any(|&(unused_index, _)| unused_index == index)
    });
    let violation = UnusedIgnoreCode {
        codes: format_by_reason(codes, &unused),
        whole_comment: deletion.whole_comment,
    };
    deletion.report(checker.context(), &violation);
}

/// Why `code` is unused, `None` when it is used or not this rule's to judge.
///
/// `matched` holds the rules the comment silenced, `earlier` the codes it lists before `code`.
fn classify(
    checker: &Checker<'_>,
    code: &str,
    earlier: &[&str],
    matched: RuleSet,
    scope: IgnoreScope,
) -> Option<Unused> {
    if let Ok(rule) = Rule::from_str(code) {
        return if earlier.contains(&code) {
            Some(Unused::Duplicated)
        } else if matched.contains(rule) {
            None
        } else if checker.is_rule_enabled(rule) {
            Some(Unused::Unmatched)
        } else {
            Some(Unused::Disabled)
        };
    }
    match ReservedCode::from_str(code) {
        // `invalid-syntax` on a node is `invalid-ignore-comment`'s, repeated or not.
        Ok(ReservedCode::InvalidSyntax) if scope == IgnoreScope::Node => None,
        Ok(_) if earlier.contains(&code) => Some(Unused::Duplicated),
        // The file parsed, so there is no syntax error left to suppress.
        Ok(ReservedCode::InvalidSyntax) => Some(Unused::Unmatched),
        // `format` speaks to the formatter, which the linter cannot see,
        // and a code naming no rule is `invalid-ignore-code`'s to report.
        Ok(ReservedCode::Format) | Err(_) => None,
    }
}

/// The unused codes grouped by reason, `; ` between groups: `` `a`; `b`, `c` (non-enabled) ``.
fn format_by_reason(codes: &[&str], unused: &[(usize, Unused)]) -> String {
    Unused::ALL
        .iter()
        .filter_map(|&reason| {
            let group = unused
                .iter()
                .filter(|&&(_, cause)| cause == reason)
                .map(|&(index, _)| format!("`{}`", codes[index]))
                .collect::<Vec<_>>();
            (!group.is_empty()).then(|| format!("{}{}", group.join(", "), reason.label()))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

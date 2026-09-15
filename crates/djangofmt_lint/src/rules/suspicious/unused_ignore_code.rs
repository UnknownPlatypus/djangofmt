use std::borrow::Cow;
use std::str::FromStr;

use crate::Checker;
use crate::fix::FixAvailability;
use crate::fix::edits::delete_codes_or_comment;
use crate::registry::{Rule, RuleCategory};
use crate::suppression::{IgnoreComment, IgnoreScope, ReservedCode};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `ignore[...]` / `file-ignore[...]` suppression that are no longer applicable.
///
/// ## Why is this bad?
/// A suppression that no longer matches any diagnostic violations is likely included by mistake,
/// and should be removed to avoid confusion.
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
/// The fix is marked as unsafe when it deletes a comment along with the free-text reason after its code list, as in `ignore[...]: reason`.
/// Dropping a code from a list, or a comment carrying no reason, is safe.
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct UnusedIgnoreCode {
    /// The unused codes, grouped by reason: `` `a`; `b`, `c` (disabled rule) ``.
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
            Self::Disabled => " (disabled rule)",
            Self::Duplicated => " (duplicated)",
        }
    }
}

/// Report the unused codes of every comment, once per comment.
pub fn check(checker: &Checker<'_>, comments: &[IgnoreComment<'_>]) {
    for comment in comments {
        check_comment(checker, comment);
    }
}

fn check_comment(checker: &Checker<'_>, comment: &IgnoreComment<'_>) {
    // Malformed and misplaced directives are `invalid-ignore-comment`'s to report.
    if comment.scope.is_none() {
        return;
    }
    let codes = comment.directive.codes();
    // A comment silencing this very rule is left alone, whatever else it lists.
    let own_code: &str = Rule::UnusedIgnoreCode.into();
    if codes.contains(&own_code) {
        return;
    }

    // One slot per code, so of a repeated code only the repeat is dropped.
    let unused: Vec<Option<Unused>> = (0..codes.len())
        .map(|index| classify(checker, comment, index))
        .collect();
    if unused.iter().all(Option::is_none) {
        return;
    }

    let deletion = delete_codes_or_comment(checker.context(), comment, |index, _| {
        unused[index].is_some()
    });
    let violation = UnusedIgnoreCode {
        codes: format_by_reason(codes, &unused),
        whole_comment: deletion.whole_comment,
    };
    deletion.report(checker.context(), &violation);
}

/// Why the code at `index` is unused, `None` when it is used or not this rule's to judge.
fn classify(checker: &Checker<'_>, comment: &IgnoreComment<'_>, index: usize) -> Option<Unused> {
    let codes = comment.directive.codes();
    let code = codes[index];
    let earlier = &codes[..index];
    if let Ok(rule) = Rule::from_str(code) {
        return if earlier.contains(&code) {
            Some(Unused::Duplicated)
        } else if comment.matched.contains(rule) {
            None
        } else if checker.is_rule_enabled(rule) {
            Some(Unused::Unmatched)
        } else {
            Some(Unused::Disabled)
        };
    }
    match ReservedCode::from_str(code) {
        // `invalid-syntax` on a node is `invalid-ignore-comment`'s, repeated or not.
        Ok(ReservedCode::InvalidSyntax) if comment.scope == Some(IgnoreScope::Node) => None,
        Ok(_) if earlier.contains(&code) => Some(Unused::Duplicated),
        // The file parsed, so there is no syntax error left to suppress.
        Ok(ReservedCode::InvalidSyntax) => Some(Unused::Unmatched),
        // `format` speaks to the formatter, which the linter cannot see,
        // and a code naming no rule is `invalid-ignore-code`'s to report.
        Ok(ReservedCode::Format) | Err(_) => None,
    }
}

/// The unused codes grouped by reason, `; ` between groups: `` `a`; `b`, `c` (disabled rule) ``.
fn format_by_reason(codes: &[&str], unused: &[Option<Unused>]) -> String {
    Unused::ALL
        .iter()
        .filter_map(|&reason| {
            let group = codes
                .iter()
                .zip(unused)
                .filter(|&(_, cause)| *cause == Some(reason))
                .map(|(code, _)| format!("`{code}`"))
                .collect::<Vec<_>>();
            (!group.is_empty()).then(|| format!("{}{}", group.join(", "), reason.label()))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

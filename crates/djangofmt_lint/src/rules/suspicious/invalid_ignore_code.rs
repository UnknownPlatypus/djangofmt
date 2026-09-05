use std::borrow::Cow;
use std::str::FromStr;

use crate::registry::{Rule, RuleCategory};
use crate::suppression::FileIgnoreCode;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};
use crate::{Checker, span};

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
    pub code: String,
}

impl Violation for InvalidIgnoreCode {
    const RULE: Rule = Rule::InvalidIgnoreCode;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        format!("Invalid rule code in suppression: `{}`", self.code).into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some("Remove unused suppression".into())
    }
}

/// Report every code naming neither a rule nor a reserved code.
pub fn check(codes: &[&str], checker: &Checker<'_>) {
    for code in codes {
        if Rule::from_str(code).is_ok() || FileIgnoreCode::from_str(code).is_ok() {
            continue;
        }
        checker.report_diagnostic(
            &InvalidIgnoreCode {
                code: (*code).to_string(),
            },
            span(checker.source_offset(code), code.len()),
        );
    }
}

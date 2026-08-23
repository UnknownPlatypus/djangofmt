use std::borrow::Cow;

use crate::registry::{Rule, RuleCategory};
use crate::suppression::{FORMAT, INVALID_SYNTAX};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};
use crate::{Checker, span};

/// ## What it does
/// Checks for `ignore[...]` / `file-ignore[...]` suppression comments listing a code that does not
/// name any rule.
///
/// ## Why is this bad?
/// An unknown code suppresses nothing, so the diagnostic the comment was meant to silence keeps
/// firing. It is usually a typo, or a leftover from a rule that was renamed.
///
/// Besides rule names, `file-ignore[...]` accepts the codes `format` (skip formatting the file) and
/// `invalid-syntax` (skip a file that does not parse).
///
/// ## Example
/// ```html
/// {# djangofmt: ignore[invalid-attr-values] #}
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
pub struct UnknownIgnoreCode {
    pub code: String,
}

impl Violation for UnknownIgnoreCode {
    const RULE: Rule = Rule::UnknownIgnoreCode;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        format!("Unknown rule code `{}` in suppression comment.", self.code).into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some("Fix the typo or remove the code: valid codes are the documented rule names.".into())
    }
}

/// Report every code naming neither a rule nor a reserved code.
pub fn check(codes: &[&str], checker: &Checker<'_>) {
    for code in codes {
        if *code == FORMAT || *code == INVALID_SYNTAX || code.parse::<Rule>().is_ok() {
            continue;
        }
        let offset = checker.source_offset(code);
        checker.report_diagnostic_if_enabled(
            &UnknownIgnoreCode {
                code: (*code).to_string(),
            },
            span(offset, code.len()),
        );
    }
}

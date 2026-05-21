use std::fmt::Debug;

pub use djangofmt_macros::ViolationMetadata;

use crate::fix::FixAvailability;
use crate::registry::{Rule, RuleCategory};

/// Static, derive-supplied metadata for a lint violation.
///
/// Implemented automatically by `#[derive(ViolationMetadata)]`, which captures the struct's `///` doc comment as the rule's explanation
/// and records the source file and line of the struct definition.
/// Powers the per-rule documentation generator under `djangofmt_dev`.
pub trait ViolationMetadata {
    /// The rendered rule documentation, taken verbatim from the violation struct's doc comment.
    fn explain() -> &'static str;

    /// The source file path of the violation struct, as produced by `file!()`.
    fn file() -> &'static str;

    /// The source line of the violation struct definition.
    fn line() -> u32;
}

/// A trait for lint violations.
///
/// This trait is implemented by structs that represent specific lint rules.
/// Each violation knows its own rule via the associated constant.
///
/// The `RULE` constant ties the violation to the registry, enabling:
/// - `checker.report(violation, span)` without passing the rule explicitly
/// - Compile-time verification that every violation has a registered rule
///
/// The `CATEGORY` constant declares the functional grouping (e.g., Correctness, Style).
///
/// The `FIX_AVAILABILITY` constant declares whether (and how often) the rule
/// produces a fix. Default is [`FixAvailability::None`] for fixless rules.
pub trait Violation: Debug {
    /// The rule for this violation (e.g., `Rule::InvalidAttrValue`).
    const RULE: Rule;

    /// The category for this violation (e.g., `RuleCategory::Correctness`).
    const CATEGORY: RuleCategory;

    /// Whether this rule produces a fix, and if so how often.
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    /// The message to be displayed to the user.
    fn message(&self) -> String;

    /// Optional help text with suggestions for fixing the issue.
    fn help(&self) -> Option<String> {
        None
    }

    /// A short, imperative summary of what the fix does (e.g. `"Add trimmed"`).
    ///
    /// Set on the diagnostic by [`crate::LintContext::report_diagnostic`].
    fn fix_title(&self) -> Option<String> {
        None
    }
}

use std::fmt::Debug;

use crate::registry::Rule;

/// A trait for lint violations.
///
/// This trait is implemented by structs that represent specific lint rules.
/// Each violation knows its own rule via the associated constant.
///
/// The `RULE` constant ties the violation to the registry, enabling:
/// - `checker.report(violation, span)` without passing the rule explicitly
/// - Compile-time verification that every violation has a registered rule
pub trait Violation: Debug {
    /// The rule for this violation (e.g., `Rule::InvalidAttrValue`).
    const RULE: Rule;

    /// The message to be displayed to the user.
    fn message(&self) -> String;

    /// Optional help text with suggestions for fixing the issue.
    fn help(&self) -> Option<String> {
        None
    }
}

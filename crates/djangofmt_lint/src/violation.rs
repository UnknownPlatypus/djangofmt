use std::fmt::Debug;

use crate::registry::{Rule, RuleCategory};
use crate::rule_group::RuleGroup;

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
/// The `GROUP` constant declares the stability grouping
/// (e.g., Stable, Preview). It is propagated to `Rule::group()` by the
/// `define_rules!` macro and consumed by [`crate::rule_selector::RuleSelector::rules`].
pub trait Violation: Debug {
    /// The rule for this violation (e.g., `Rule::InvalidAttrValue`).
    const RULE: Rule;

    /// The category for this violation (e.g., `RuleCategory::Correctness`).
    const CATEGORY: RuleCategory;

    /// The stability group for this violation (e.g., `RuleGroup::Stable`).
    const GROUP: RuleGroup;

    /// The message to be displayed to the user.
    fn message(&self) -> String;

    /// Optional help text with suggestions for fixing the issue.
    fn help(&self) -> Option<String> {
        None
    }
}

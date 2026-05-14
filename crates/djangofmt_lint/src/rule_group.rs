//! Per-rule stability grouping.
//!
//! Mirrors ruff's `RuleGroup` (`crates/ruff_linter/src/codes.rs`). The group
//! gates whether a rule is enabled by default, requires `--preview`, or has
//! been retired.
//!
//! The group is attached to each rule via the [`Violation::GROUP`](crate::violation::Violation::GROUP)
//! associated constant and surfaced on [`Rule::group`](crate::registry::Rule::group)
//! via the `define_rules!` macro.

/// Stability classification for a lint rule.
///
/// Determines whether `RuleSelector::All` and category selectors include the
/// rule by default, as well as how `--preview` interacts with selection.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RuleGroup {
    /// The rule is enabled by default and considered stable.
    Stable,
    /// The rule is unstable; it must be opted into via `--preview` or by
    /// referencing it through an exact (rule-level) selector.
    Preview,
    /// The rule has been deprecated. It is only included when explicitly
    /// selected by an exact selector while preview mode is disabled.
    Deprecated,
    /// The rule has been removed. It is only "selectable" through an exact
    /// selector so users can keep ignoring it during the rename grace period.
    Removed,
}

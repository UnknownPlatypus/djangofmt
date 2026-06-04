//! Resolved linter configuration.
//!
//! [`Settings`] is the immutable, fully-resolved configuration consumed by
//! [`crate::check_ast`]. It carries the active [`RuleSet`] (bitset of enabled
//! rules) and the [`PreviewMode`] used to filter preview rules during
//! resolution.
//!
//! Higher layers (CLI / pyproject) construct one [`RuleSelection`] per
//! configuration source and call [`Settings::from_selections`] to fold them
//! into a [`RuleSet`] using specificity ordering: broadest
//! selectors (`ALL`) apply first, then category-level, then exact rule
//! selectors. At each level selects / extends are applied additively and
//! ignores are applied subtractively, so `select = ["ALL"], ignore =
//! ["invalid-attr-value"]` resolves correctly without ad-hoc precedence logic.
//! [`Settings::from_selectors`] is a single-layer convenience that delegates
//! to [`Settings::from_selections`].

use strum::IntoEnumIterator;

use crate::registry::Rule;
use crate::rule_selector::{RuleSelector, Specificity};
use crate::rule_set::RuleSet;

/// Whether preview (unstable) rules are eligible for selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreviewMode {
    #[default]
    Disabled,
    Enabled,
}

impl From<bool> for PreviewMode {
    fn from(enabled: bool) -> Self {
        if enabled {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

/// A single rule-selection layer.
///
/// A layer carries the four selector lists that the user can express in one
/// "place" (defaults, pyproject, CLI flags). The `select` slot is
/// `Option<&[...]>` so the caller can distinguish a missing `select`
/// (`None` — extend behavior) from an explicit empty list (`Some(&[])` —
/// replacement with zero rules).
#[derive(Debug, Clone, Copy, Default)]
pub struct RuleSelection<'a> {
    pub select: Option<&'a [RuleSelector]>,
    pub ignore: &'a [RuleSelector],
    pub extend_select: &'a [RuleSelector],
    pub extend_ignore: &'a [RuleSelector],
}

/// Configuration settings for the linter.
#[derive(Debug, Clone)]
pub struct Settings {
    /// The set of rules that are active for this run.
    pub rules: RuleSet,
    /// Whether preview-stability rules are eligible for selection.
    pub preview: PreviewMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self::all_rules(PreviewMode::Disabled)
    }
}

/// The implicit base layer's `select` list (`["ALL"]`).
const DEFAULT_SELECTORS: &[RuleSelector] = &[RuleSelector::All];

impl Settings {
    /// Build [`Settings`] from a single layer of selectors.
    ///
    /// A convenience wrapper over [`Settings::from_selections`] for callers
    /// (defaults, tests, one-shot embedders) that have exactly one layer. The
    /// selectors are wrapped in a single [`RuleSelection`] whose `Some(select)`
    /// *replaces* the implicit `ALL` base, so the result is just this layer
    /// resolved with specificity ordering — `select`/`extend_select`
    /// additive, `ignore`/`extend_ignore` subtractive, narrower selectors
    /// winning.
    #[must_use]
    pub fn from_selectors(
        select: &[RuleSelector],
        ignore: &[RuleSelector],
        extend_select: &[RuleSelector],
        extend_ignore: &[RuleSelector],
        preview: PreviewMode,
    ) -> Self {
        Self::from_selections(
            &[RuleSelection {
                select: Some(select),
                ignore,
                extend_select,
                extend_ignore,
            }],
            preview,
        )
    }

    /// Build [`Settings`] with every registered rule selected.
    ///
    /// Convenience for embedders, benchmarks, and tests that want the full
    /// rule set; `preview` controls whether preview-stability rules are
    /// included.
    #[must_use]
    pub fn all_rules(preview: PreviewMode) -> Self {
        Self::from_selectors(&[RuleSelector::All], &[], &[], &[], preview)
    }

    /// Build [`Settings`] from a sequence of [`RuleSelection`] layers.
    ///
    /// Folds a sequence of selection layers into a single rule set. An
    /// implicit base layer (`select = ["ALL"]`) is prepended
    /// so the very first layer's `extend_select` / `ignore` behave the way
    /// users expect (additive on the default rule set). The layers in
    /// `selections` are then applied in order, each one either *replacing*
    /// the running set (when it has `Some(select)`) or *extending* it
    /// (when `select` is `None`).
    #[must_use]
    pub fn from_selections(selections: &[RuleSelection<'_>], preview: PreviewMode) -> Self {
        let default_layer = RuleSelection {
            select: Some(DEFAULT_SELECTORS),
            ignore: &[],
            extend_select: &[],
            extend_ignore: &[],
        };

        let mut rules = RuleSet::empty();
        for layer in std::iter::once(&default_layer).chain(selections.iter()) {
            rules = Self::apply_layer(rules, layer, preview);
        }

        Self { rules, preview }
    }

    /// Apply a single [`RuleSelection`] layer to a running [`RuleSet`].
    ///
    /// Per-specificity loop matches `Settings::from_selectors`: at each
    /// specificity level, `select`/`extend_select` apply additively, then
    /// `ignore`/`extend_ignore` apply subtractively. If the layer has
    /// `Some(select)`, the running set is rebuilt from this layer's updates
    /// (dropping prior state, including ignores). Otherwise the updates are
    /// merged on top.
    fn apply_layer(running: RuleSet, layer: &RuleSelection<'_>, preview: PreviewMode) -> RuleSet {
        let select_iter = || layer.select.iter().copied().flatten();
        let extend_select_iter = || layer.extend_select.iter();
        let ignore_iter = || layer.ignore.iter();
        let extend_ignore_iter = || layer.extend_ignore.iter();

        // A layer with `Some(select)` rebuilds from scratch (replacement
        // semantics, dropping prior state including ignores); otherwise it
        // merges onto the running set. The base is fixed before any update, so
        // selects/ignores can apply directly per specificity level.
        let mut next = if layer.select.is_some() {
            RuleSet::empty()
        } else {
            running
        };

        for spec in Specificity::iter() {
            for selector in select_iter()
                .chain(extend_select_iter())
                .filter(|s| s.specificity() == spec)
            {
                for rule in selector.rules(preview) {
                    next.insert(rule);
                }
            }
            for selector in ignore_iter()
                .chain(extend_ignore_iter())
                .filter(|s| s.specificity() == spec)
            {
                for rule in selector.rules(preview) {
                    next.remove(rule);
                }
            }
        }
        next
    }

    /// Check if a specific rule is enabled.
    #[must_use]
    #[inline]
    pub const fn is_enabled(&self, rule: Rule) -> bool {
        self.rules.contains(rule)
    }

    /// Check if any of the given rules is enabled.
    ///
    /// Cheap pre-filter for a cluster of related rules: each test is a bitset
    /// lookup, so a fully-disabled cluster can be skipped with one call.
    #[must_use]
    #[inline]
    pub const fn any_rule_enabled(&self, rules: &[Rule]) -> bool {
        let mut i = 0;
        while i < rules.len() {
            if self.is_enabled(rules[i]) {
                return true;
            }
            i += 1;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{PreviewMode, Settings};
    use crate::registry::Rule;
    use crate::rule_set::RuleSet;

    #[test]
    fn any_rule_enabled_reflects_membership() {
        let all = Settings::default();
        assert!(all.any_rule_enabled(&[Rule::UseHttps]));
        assert!(all.any_rule_enabled(&[Rule::UseHttps, Rule::InvalidAttrValue]));
        assert!(!all.any_rule_enabled(&[]));

        let none = Settings {
            rules: RuleSet::empty(),
            preview: PreviewMode::Disabled,
        };
        assert!(!none.any_rule_enabled(&[Rule::UseHttps, Rule::InvalidAttrValue]));

        let partial = Settings {
            rules: RuleSet::from_rule(Rule::UseHttps),
            preview: PreviewMode::Disabled,
        };
        assert!(partial.any_rule_enabled(&[Rule::UseHttps, Rule::InvalidAttrValue]));
        assert!(!partial.any_rule_enabled(&[Rule::InvalidAttrValue]));
    }
}

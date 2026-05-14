//! Resolved linter configuration.
//!
//! [`Settings`] is the immutable, fully-resolved configuration consumed by
//! [`crate::check_ast`]. It carries the active [`RuleSet`] (bitset of enabled
//! rules) and the [`PreviewMode`] used to filter preview rules during
//! resolution.
//!
//! Higher layers (CLI / pyproject) construct selectors and call
//! [`Settings::from_selectors`] to fold them into a [`RuleSet`] using
//! ruff-style specificity ordering: broadest selectors (`ALL`) apply first,
//! then category-level, then exact rule selectors. At each level selects /
//! extends are applied additively and ignores are applied subtractively, so
//! `select = ["ALL"], ignore = ["invalid-attr-value"]` resolves correctly
//! without ad-hoc precedence logic.

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
/// Mirrors ruff's `RuleSelection` (`crates/ruff_workspace/src/configuration.rs`).
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
        Self::from_selectors(&[RuleSelector::All], &[], &[], &[], PreviewMode::Disabled)
    }
}

/// The implicit base layer's `select` list (`["ALL"]`).
const DEFAULT_SELECTORS: &[RuleSelector] = &[RuleSelector::All];

impl Settings {
    /// Build [`Settings`] by resolving a single layer of selectors.
    ///
    /// Resolution mirrors ruff's `LintConfiguration::as_rule_table` for a
    /// single layer:
    ///
    /// 1. Iterate [`Specificity`] from broadest (`All`) to narrowest (`Rule`).
    /// 2. At each level, apply `select`/`extend_select` additively, then
    ///    `ignore`/`extend_ignore` subtractively.
    ///
    /// `select` and `extend_select` are treated identically at this layer —
    /// the replacement-vs-extend distinction belongs to the multi-layer
    /// resolver in the binary crate, which decides whether a layer's
    /// `Some(select)` *replaces* the carried set before calling this method.
    #[must_use]
    pub fn from_selectors(
        select: &[RuleSelector],
        ignore: &[RuleSelector],
        extend_select: &[RuleSelector],
        extend_ignore: &[RuleSelector],
        preview: PreviewMode,
    ) -> Self {
        let mut rules = RuleSet::empty();

        for spec in Specificity::iter() {
            for selector in select
                .iter()
                .chain(extend_select.iter())
                .filter(|s| s.specificity() == spec)
            {
                rules.extend(selector.rules(preview));
            }
            for selector in ignore
                .iter()
                .chain(extend_ignore.iter())
                .filter(|s| s.specificity() == spec)
            {
                for rule in selector.rules(preview) {
                    rules.remove(rule);
                }
            }
        }

        Self { rules, preview }
    }

    /// Build [`Settings`] from a sequence of [`RuleSelection`] layers.
    ///
    /// Mirrors ruff's `LintConfiguration::as_rule_table` multi-layer
    /// resolution. An implicit base layer (`select = ["ALL"]`) is prepended
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

        let mut updates: Vec<(Rule, bool)> = Vec::new();
        for spec in Specificity::iter() {
            for selector in select_iter()
                .chain(extend_select_iter())
                .filter(|s| s.specificity() == spec)
            {
                for rule in selector.rules(preview) {
                    updates.push((rule, true));
                }
            }
            for selector in ignore_iter()
                .chain(extend_ignore_iter())
                .filter(|s| s.specificity() == spec)
            {
                for rule in selector.rules(preview) {
                    updates.push((rule, false));
                }
            }
        }

        let mut next = if layer.select.is_some() {
            // Replacement semantics: drop everything carried so far.
            RuleSet::empty()
        } else {
            running
        };
        for (rule, enabled) in updates {
            if enabled {
                next.insert(rule);
            } else {
                next.remove(rule);
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
}

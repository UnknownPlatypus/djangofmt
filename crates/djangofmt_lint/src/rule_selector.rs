//! User-facing rule selectors (e.g. `ALL`, `correctness`, `invalid-attr-value`).
//!
//! Mirrors ruff's `RuleSelector` (`crates/ruff_linter/src/rule_selector.rs`)
//! at djangofmt scale. djangofmt has no rule codes or linter-prefix taxonomy,
//! so selectors are limited to:
//!
//! - [`RuleSelector::All`] — every registered rule
//! - [`RuleSelector::Category`] — every rule of a given functional category
//! - [`RuleSelector::Rule`] — a single rule, addressed by its kebab-case name
//!
//! The two-step expansion (`all_rules` then `rules(preview)`) mirrors ruff so
//! that the `Deprecated`/`Removed` stability paths fit cleanly without further
//! refactoring once we start using them.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

use crate::registry::{Rule, RuleCategory, RuleGroup};
use crate::settings::PreviewMode;

/// A selector that expands into zero or more concrete [`Rule`]s.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuleSelector {
    /// Select every registered rule.
    All,
    /// Select every rule belonging to the given category.
    Category(RuleCategory),
    /// Select a single rule by its kebab-case name.
    Rule(Rule),
}

/// Specificity of a [`RuleSelector`] within a resolution layer.
///
/// Resolution iterates from broadest (`All`) to narrowest (`Rule`); at each
/// step, selects/extends are applied additively and ignores subtractively.
/// The ordering on this enum (broad < narrow) drives that traversal.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, EnumIter)]
pub enum Specificity {
    /// `ALL`-style selectors.
    All,
    /// Category selectors (e.g. `correctness`).
    Category,
    /// Exact-rule selectors (e.g. `invalid-attr-value`).
    Rule,
}

/// Error returned by [`RuleSelector::from_str`] for unparsable input.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseError {
    /// The given string did not match `ALL`, any known rule, or any known category.
    #[error("Unknown rule selector: `{0}`")]
    Unknown(String),
}

impl RuleSelector {
    /// Returns the [`Specificity`] of this selector.
    #[must_use]
    pub const fn specificity(&self) -> Specificity {
        match self {
            Self::All => Specificity::All,
            Self::Category(_) => Specificity::Category,
            Self::Rule(_) => Specificity::Rule,
        }
    }

    /// Returns `true` if this selector targets a single rule by name.
    #[must_use]
    pub const fn is_exact(&self) -> bool {
        matches!(self, Self::Rule(_))
    }

    /// All rules matched by this selector, regardless of stability group.
    ///
    /// Use [`Self::rules`] when applying preview/stability filtering.
    #[must_use]
    pub fn all_rules(&self) -> Box<dyn Iterator<Item = Rule> + '_> {
        match self {
            Self::All => Box::new(Rule::iter()),
            Self::Category(category) => {
                let category = *category;
                Box::new(Rule::iter().filter(move |r| r.category() == category))
            }
            Self::Rule(rule) => Box::new(std::iter::once(*rule)),
        }
    }

    /// Rules matched by this selector after applying stability filtering.
    ///
    /// Filtering rules:
    /// - [`RuleGroup::Stable`] — always included
    /// - [`RuleGroup::Preview`] — included if `preview == Enabled` *or* the
    ///   selector is exact (the exact-selector escape hatch matches ruff's
    ///   default `require_explicit = false` behavior)
    /// - [`RuleGroup::Deprecated`] — only when `preview == Disabled` *and* the
    ///   selector is exact
    /// - [`RuleGroup::Removed`] — only when the selector is exact
    #[must_use]
    pub fn rules(&self, preview: PreviewMode) -> Box<dyn Iterator<Item = Rule> + '_> {
        let is_exact = self.is_exact();
        let preview_enabled = preview == PreviewMode::Enabled;

        Box::new(self.all_rules().filter(move |rule| match rule.group() {
            RuleGroup::Stable { .. } => true,
            RuleGroup::Preview { .. } => preview_enabled || is_exact,
            RuleGroup::Deprecated { .. } => !preview_enabled && is_exact,
            RuleGroup::Removed { .. } => is_exact,
        }))
    }
}

impl FromStr for RuleSelector {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Order matters: try literal `ALL` first, then exact rule names, then
        // category names. This keeps the lookup unambiguous even if a future
        // rule name happens to collide with a category name.
        if s == "ALL" {
            return Ok(Self::All);
        }
        if let Ok(rule) = Rule::from_str(s) {
            return Ok(Self::Rule(rule));
        }
        if let Ok(category) = RuleCategory::from_str(s) {
            return Ok(Self::Category(category));
        }
        Err(ParseError::Unknown(s.to_string()))
    }
}

impl Display for RuleSelector {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => f.write_str("ALL"),
            Self::Category(c) => Display::fmt(c, f),
            Self::Rule(r) => Display::fmt(r, f),
        }
    }
}

impl Serialize for RuleSelector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RuleSelector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(SelectorVisitor)
    }
}

struct SelectorVisitor;

impl Visitor<'_> for SelectorVisitor {
    type Value = RuleSelector;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(
            "expected a rule selector: `ALL`, a category name, or a kebab-case rule name",
        )
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        FromStr::from_str(v).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_accepts_all_keyword() {
        assert_eq!(RuleSelector::from_str("ALL").unwrap(), RuleSelector::All);
    }

    #[test]
    fn from_str_accepts_rule_name() {
        assert_eq!(
            RuleSelector::from_str("invalid-attr-value").unwrap(),
            RuleSelector::Rule(Rule::InvalidAttrValue),
        );
    }

    #[test]
    fn from_str_accepts_category_name() {
        assert_eq!(
            RuleSelector::from_str("correctness").unwrap(),
            RuleSelector::Category(RuleCategory::Correctness),
        );
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = RuleSelector::from_str("nope").unwrap_err();
        assert!(matches!(err, ParseError::Unknown(ref s) if s == "nope"));
    }

    #[test]
    fn from_str_rejects_lowercase_all() {
        // We intentionally only accept the canonical `ALL`; "all" is treated
        // as an unknown rule/category name.
        let err = RuleSelector::from_str("all").unwrap_err();
        assert!(matches!(err, ParseError::Unknown(_)));
    }

    #[test]
    fn display_roundtrips() {
        for selector in [
            RuleSelector::All,
            RuleSelector::Category(RuleCategory::Correctness),
            RuleSelector::Rule(Rule::InvalidAttrValue),
        ] {
            let s = selector.to_string();
            let parsed = RuleSelector::from_str(&s).unwrap();
            assert_eq!(parsed, selector);
        }
    }

    #[test]
    fn specificity_ordering_is_broad_to_narrow() {
        assert!(Specificity::All < Specificity::Category);
        assert!(Specificity::Category < Specificity::Rule);
    }

    #[test]
    fn specificity_iter_traverses_broad_to_narrow() {
        let collected: Vec<_> = Specificity::iter().collect();
        assert_eq!(
            collected,
            vec![Specificity::All, Specificity::Category, Specificity::Rule]
        );
    }

    #[test]
    fn all_rules_includes_invalid_attr_value() {
        let rules: Vec<_> = RuleSelector::All.all_rules().collect();
        assert!(rules.contains(&Rule::InvalidAttrValue));
    }

    #[test]
    fn category_filters_by_category() {
        let rules: Vec<_> = RuleSelector::Category(RuleCategory::Correctness)
            .all_rules()
            .collect();
        assert!(rules.contains(&Rule::InvalidAttrValue));
        // Complexity has no rules yet.
        let no_rules: Vec<_> = RuleSelector::Category(RuleCategory::Complexity)
            .all_rules()
            .collect();
        assert!(no_rules.is_empty());
    }

    #[test]
    fn rule_selector_yields_single_rule() {
        let rules: Vec<_> = RuleSelector::Rule(Rule::InvalidAttrValue)
            .all_rules()
            .collect();
        assert_eq!(rules, vec![Rule::InvalidAttrValue]);
    }

    #[test]
    fn rules_filter_includes_stable_with_preview_disabled() {
        let rules: Vec<_> = RuleSelector::All.rules(PreviewMode::Disabled).collect();
        assert!(rules.contains(&Rule::InvalidAttrValue));
    }

    #[test]
    fn is_exact_only_for_rule_variant() {
        assert!(!RuleSelector::All.is_exact());
        assert!(!RuleSelector::Category(RuleCategory::Correctness).is_exact());
        assert!(RuleSelector::Rule(Rule::InvalidAttrValue).is_exact());
    }
}

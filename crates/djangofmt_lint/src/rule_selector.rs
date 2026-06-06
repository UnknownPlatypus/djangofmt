//! Rule selectors: the user-facing grammar for enabling and disabling lint rules.
//!
//! A selector names either:
//!  - a single rule (e.g. `invalid-attr-value`)
//!  - a group via the `category:` prefix (e.g. `category:all`, `category:correctness`, …).
//!
//! Selectors are parsed from CLI arguments and the `[tool.djangofmt.lint]` config, then resolved
//! into a [`RuleSet`](crate::rule_set::RuleSet) by [`RuleSelection`](crate::settings::RuleSelection).

use std::fmt;
use std::str::FromStr;

use strum::{IntoEnumIterator, VariantNames};

use crate::RuleGroup;
use crate::registry::{Rule, RuleCategory};

/// Prefix that marks a group (a category, or `all`) selector.
const CATEGORY_PREFIX: &str = "category:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSelector {
    /// Select all rules (includes rules in preview if enabled)
    All,
    /// Select every rule in one category (`category:<name>`).
    Category(RuleCategory),
    /// Select a single rule by its kebab-case name.
    Rule(Rule),
}

impl RuleSelector {
    /// Selection specificity used to resolve conflicts.
    /// A more specific selector wins over a less specific one regardless of ordering.
    #[must_use]
    pub const fn specificity(self) -> u8 {
        match self {
            Self::All => 0,
            Self::Category(_) => 1,
            Self::Rule(_) => 2,
        }
    }

    /// Return all matching rules, regardless of rule group filters like preview and deprecated.
    pub fn all_rules(self) -> impl Iterator<Item = Rule> {
        Rule::iter().filter(move |rule| match self {
            Self::All => true,
            Self::Category(category) => rule.category() == category,
            Self::Rule(selected) => *rule == selected,
        })
    }

    /// Returns rules matching the selector, taking into account rule groups like preview and deprecated.
    pub fn rules(self, preview: bool) -> impl Iterator<Item = Rule> {
        let exact = self.is_exact();
        self.all_rules().filter(move |rule| {
            match rule.group() {
                // Always include stable rules
                RuleGroup::Stable { .. } => true,
                // Enabling preview includes all preview rules
                RuleGroup::Preview { .. } => preview,
                // Deprecated rules are excluded by default unless explicitly selected
                RuleGroup::Deprecated { .. } => exact,
                // Never run; the resolver warns for exact selectors
                RuleGroup::Removed { .. } => false,
            }
        })
    }

    /// Returns true if this selector is exact i.e. selects a single rule by code
    #[must_use]
    pub const fn is_exact(self) -> bool {
        matches!(self, Self::Rule(_))
    }
}

impl fmt::Display for RuleSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, "{CATEGORY_PREFIX}all"),
            Self::Category(category) => write!(f, "{CATEGORY_PREFIX}{category}"),
            Self::Rule(rule) => write!(f, "{rule}"),
        }
    }
}

impl FromStr for RuleSelector {
    type Err = SelectorParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Some(group) = value.strip_prefix(CATEGORY_PREFIX) {
            return match group {
                "all" => Ok(Self::All),
                _ => RuleCategory::from_str(group)
                    .map(Self::Category)
                    .map_err(|_| SelectorParseError::unknown_category(group)),
            };
        }
        // A bare token is a rule name.
        Rule::from_str(value).map(Self::Rule).map_err(|_| {
            if value == "all" || RuleCategory::from_str(value).is_ok() {
                // The common mistake is forgetting the `category:` prefix,
                // so detect a bare category name and suggest the fix.
                SelectorParseError::missing_category_prefix(value)
            } else {
                SelectorParseError::unknown_rule(value)
            }
        })
    }
}

/// Error returned when a [`RuleSelector`] string can't be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorParseError {
    message: String,
}

impl SelectorParseError {
    /// An invalid rule selected (e.g. `--select yay`)
    fn unknown_rule(value: &str) -> Self {
        Self {
            message: format!("Unknown rule selector: `{value}`"),
        }
    }
    /// An invalid category selected (e.g. `--select category:yay`)
    fn unknown_category(value: &str) -> Self {
        Self {
            message: format!(
                "Unknown category `{value}` (expected one of: {})",
                category_list()
            ),
        }
    }

    /// An invalid rule selected that look like a category (e.g. `--select correctness`, expected `--select category:correctness`)
    fn missing_category_prefix(value: &str) -> Self {
        Self {
            message: format!("Unknown rule `{value}`; did you mean `{CATEGORY_PREFIX}{value}`?"),
        }
    }
}

impl fmt::Display for SelectorParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SelectorParseError {}

/// Comma-joined list of valid group names (`all` plus the categories), for error messages.
#[must_use]
pub fn category_list() -> String {
    std::iter::once("all")
        .chain(RuleCategory::VARIANTS.iter().copied())
        .collect::<Vec<_>>()
        .join(", ")
}

/// A non-fatal issue surfaced while resolving a [`RuleSelection`](crate::settings::RuleSelection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionWarning {
    /// A preview rule was named explicitly without preview mode; it was skipped.
    PreviewRuleSkipped(Rule),
    /// A deprecated rule was selected explicitly.
    DeprecatedRuleSelected(Rule),
    /// A removed rule was selected explicitly; it was skipped.
    RemovedRuleSelected(Rule),
}

impl fmt::Display for SelectionWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PreviewRuleSkipped(rule) => write!(
                f,
                "rule `{rule}` is in preview and was skipped; enable it with `--preview`"
            ),
            Self::DeprecatedRuleSelected(rule) => write!(f, "rule `{rule}` is deprecated"),
            Self::RemovedRuleSelected(rule) => {
                write!(f, "rule `{rule}` has been removed and was skipped")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RuleSelector;
    use crate::registry::Rule;

    /// `all_rules` is total: an exact selector denotes exactly its rule,
    /// regardless of lifecycle. This is what docs/`--explain` output relies on.
    #[test]
    fn exact_selector_denotes_its_rule() {
        // A stable rule.
        let stable = RuleSelector::Rule(Rule::InvalidAttrValue);
        assert!(stable.is_exact());
        assert_eq!(
            stable.all_rules().collect::<Vec<_>>(),
            vec![Rule::InvalidAttrValue]
        );
        assert_eq!(
            stable.rules(false).collect::<Vec<_>>(),
            vec![Rule::InvalidAttrValue]
        );

        // A preview rule is still denoted by `all_rules`, but only *runs* under preview.
        let preview = RuleSelector::Rule(Rule::EmptyTagPair);
        assert_eq!(
            preview.all_rules().collect::<Vec<_>>(),
            vec![Rule::EmptyTagPair]
        );
        assert!(preview.rules(false).next().is_none());
        assert_eq!(
            preview.rules(true).collect::<Vec<_>>(),
            vec![Rule::EmptyTagPair]
        );
    }

    /// Group selectors are not exact, so lifecycle filtering applies: preview
    /// rules only surface under preview, deprecated rules never (require explicit
    /// selection).
    #[test]
    fn group_selector_gates_preview() {
        let all = RuleSelector::All;
        assert!(!all.is_exact());
        // Denotation ignores lifecycle…
        assert!(all.all_rules().any(|rule| rule == Rule::EmptyTagPair));
        // …but a non-preview run excludes the preview rule.
        assert!(!all.rules(false).any(|rule| rule == Rule::EmptyTagPair));
        assert!(all.rules(true).any(|rule| rule == Rule::EmptyTagPair));
    }
}

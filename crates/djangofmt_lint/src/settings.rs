use strum::IntoEnumIterator;

use crate::registry::Rule;
use crate::rule_set::RuleSet;

/// Configuration settings for the linter.
#[derive(Debug, Clone)]
pub struct Settings {
    /// The set of rules that are active for this run.
    pub rules: RuleSet,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rules: Rule::iter().collect(),
        }
    }
}

impl Settings {
    /// Check if a specific rule is enabled.
    #[must_use]
    #[inline]
    pub const fn is_enabled(&self, rule: Rule) -> bool {
        self.rules.contains(rule)
    }
}

use rustc_hash::FxHashSet;
use strum::IntoEnumIterator;

use crate::registry::Rule;

/// Configuration settings for the linter.
#[derive(Debug, Clone)]
pub struct Settings {
    /// The set of rules that are active for this run.
    pub rules: FxHashSet<Rule>,
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
    pub fn is_enabled(&self, rule: Rule) -> bool {
        self.rules.contains(&rule)
    }
}

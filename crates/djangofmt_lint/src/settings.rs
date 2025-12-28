use rustc_hash::FxHashSet;

use crate::registry::RuleCode;
use strum::IntoEnumIterator;

/// Configuration settings for the linter.
#[derive(Debug, Clone)]
pub struct Settings {
    /// The set of rules that are active for this run.
    pub rules: FxHashSet<RuleCode>,
}

impl Default for Settings {
    fn default() -> Self {
        let mut rules = FxHashSet::default();
        // Default: Enable ALL rules as requested.
        for rule in RuleCode::iter() {
            rules.insert(rule);
        }
        Self { rules }
    }
}

impl Settings {
    /// Check if a specific rule is enabled.
    #[must_use]
    #[inline]
    pub fn is_enabled(&self, code: RuleCode) -> bool {
        self.rules.contains(&code)
    }
}

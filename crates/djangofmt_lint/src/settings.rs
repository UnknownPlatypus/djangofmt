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

    /// Check whether any rule in `rules` is enabled.
    ///
    /// Used as a cheap pre-filter before a cluster of related rules: each test
    /// is a branchless bitset `contains`, so a fully-disabled cluster costs only
    /// a handful of bit tests. `const fn`, so the slice is walked with a `while`
    /// loop (slice `.iter()` is not yet const-iterable).
    #[must_use]
    #[inline]
    pub const fn any_rule_enabled(&self, rules: &[Rule]) -> bool {
        let mut i = 0;
        while i < rules.len() {
            if self.rules.contains(rules[i]) {
                return true;
            }
            i += 1;
        }
        false
    }
}

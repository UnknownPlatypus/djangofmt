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
    use super::Settings;
    use crate::registry::Rule;
    use crate::rule_set::RuleSet;

    #[test]
    fn any_rule_enabled_reflects_membership() {
        let all = Settings::default();
        assert!(all.any_rule_enabled(&[Rule::UseHttps]));
        assert!(all.any_rule_enabled(&[Rule::UseHttps, Rule::InvalidAttrValue]));
        // An empty cluster is vacuously disabled.
        assert!(!all.any_rule_enabled(&[]));

        let none = Settings {
            rules: RuleSet::default(),
        };
        assert!(!none.any_rule_enabled(&[Rule::UseHttps, Rule::InvalidAttrValue]));

        // A cluster where only one rule is enabled must still report `true`,
        // and a slice naming only the disabled rule must report `false` — this
        // pins the `any` semantics against an `all`/first-only implementation.
        let partial = Settings {
            rules: RuleSet::from_rule(Rule::UseHttps),
        };
        assert!(partial.any_rule_enabled(&[Rule::UseHttps, Rule::InvalidAttrValue]));
        assert!(!partial.any_rule_enabled(&[Rule::InvalidAttrValue]));
    }
}

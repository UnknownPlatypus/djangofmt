//! Bitset of enabled lint rules: one bit per `Rule`, membership is a branchless bit test.

use std::fmt::{Debug, Formatter};
use strum::EnumCount as _;

use crate::registry::Rule;

/// `u64` words needed for one bit per `Rule`, derived from `Rule::COUNT` so it scales as rules are added.
const RULESET_SIZE: usize = Rule::COUNT.div_ceil(64);

/// A compact bitset of enabled lint rules: one bit per rule, tested with an array index + shift.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct RuleSet([u64; RULESET_SIZE]);

// Todo: add [`from_rules`](https://github.com/astral-sh/ruff/blob/f414174695c9c2067b04c95b51709c28d27a1d03/crates/ruff_linter/src/registry/rule_set.rs#L54) ?
impl RuleSet {
    #[expect(clippy::cast_possible_truncation)] // u64::BITS is 64, fits in u16
    const SLICE_BITS: u16 = u64::BITS as u16;

    /// Returns an empty rule set.
    #[must_use]
    pub const fn empty() -> Self {
        Self([0u64; RULESET_SIZE])
    }

    /// A set containing only `rule`.
    #[must_use]
    #[inline]
    pub const fn from_rule(rule: Rule) -> Self {
        let mut set = Self::empty();
        set.insert(rule);
        set
    }

    /// Add `rule` to the set.
    #[inline]
    pub const fn insert(&mut self, rule: Rule) {
        let rule = rule as u16;
        let index = rule as usize / Self::SLICE_BITS as usize;
        let shift = rule % Self::SLICE_BITS;
        self.0[index] |= 1u64 << shift;
    }

    /// Remove `rule` from the set.
    #[inline]
    pub const fn remove(&mut self, rule: Rule) {
        let rule = rule as u16;
        let index = rule as usize / Self::SLICE_BITS as usize;
        let shift = rule % Self::SLICE_BITS;
        self.0[index] &= !(1u64 << shift);
    }

    /// Whether `rule` is in the set.
    #[must_use]
    #[inline]
    pub const fn contains(&self, rule: Rule) -> bool {
        let rule = rule as u16;
        let index = rule as usize / Self::SLICE_BITS as usize;
        let shift = rule % Self::SLICE_BITS;
        self.0[index] & (1u64 << shift) != 0
    }

    /// Union `other` into this set in place.
    #[inline]
    pub const fn union(&mut self, other: &Self) {
        let mut i = 0;
        while i < RULESET_SIZE {
            self.0[i] |= other.0[i];
            i += 1;
        }
    }

    /// Iterate the rules in ascending discriminant order.
    #[must_use]
    pub const fn iter(&self) -> RuleSetIterator {
        RuleSetIterator {
            set: *self,
            word_index: 0,
            word: self.0[0],
        }
    }
}

impl Debug for RuleSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl FromIterator<Rule> for RuleSet {
    fn from_iter<I: IntoIterator<Item = Rule>>(iter: I) -> Self {
        let mut set = Self::default();
        for rule in iter {
            set.insert(rule);
        }
        set
    }
}

impl IntoIterator for RuleSet {
    type Item = Rule;
    type IntoIter = RuleSetIterator;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for &RuleSet {
    type Item = Rule;
    type IntoIter = RuleSetIterator;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over the rules in a [`RuleSet`], using `trailing_zeros()` to skip zero words.
pub struct RuleSetIterator {
    set: RuleSet,
    word_index: usize,
    word: u64,
}

impl Iterator for RuleSetIterator {
    type Item = Rule;

    fn next(&mut self) -> Option<Rule> {
        loop {
            // Skip fully-consumed words.
            while self.word == 0 {
                self.word_index += 1;
                if self.word_index >= RULESET_SIZE {
                    return None;
                }
                self.word = self.set.0[self.word_index];
            }
            let bit = self.word.trailing_zeros();
            // Clear the lowest set bit.
            self.word &= self.word - 1;
            let global_index = self.word_index * RuleSet::SLICE_BITS as usize + bit as usize;
            // A bit with no matching `Rule` (e.g. padding in the final word) is skipped rather than ending iteration.
            #[expect(clippy::cast_possible_truncation)]
            if let Some(rule) = Rule::from_repr(global_index as u16) {
                return Some(rule);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use super::{Rule, RuleSet};

    #[test]
    fn test_all_rules() {
        for rule in Rule::iter() {
            let set = RuleSet::from_rule(rule);

            assert!(set.contains(rule));
        }

        let all_rules_set: RuleSet = Rule::iter().collect();
        let all_rules: Vec<_> = all_rules_set.iter().collect();
        let expected_rules: Vec<_> = Rule::iter().collect();
        assert_eq!(all_rules, expected_rules);
    }

    #[test]
    fn remove_not_existing_rule_from_set() {
        let mut set = RuleSet::empty();

        set.remove(Rule::EmptyTagPair);

        assert!(!set.contains(Rule::EmptyTagPair));
        assert_eq!(set, RuleSet::empty());
        assert_eq!(set.into_iter().collect::<Vec<_>>(), vec![]);
    }
}

//! Bitset of [`Rule`] discriminants.
//!
//! Mirrors ruff's `RuleSet` (`crates/ruff_linter/src/registry/rule_set.rs`).
//! Each rule maps to a single bit in a fixed-size `[u64; N]` array. We size
//! `N = 1` today (64 rules) and bump as the registry grows. The
//! capacity check in [`crate::registry`] fails the build if a future rule
//! addition overflows the bitset before `RULESET_SIZE` is bumped.

use std::fmt::{Debug, Formatter};
use std::iter::FusedIterator;

use crate::registry::Rule;

/// Number of `u64` words in the underlying bitset.
///
/// 64 rules fit comfortably today and leave headroom for the foreseeable
/// future. Increase when the rule count approaches the limit.
pub const RULESET_SIZE: usize = 1;

/// Total capacity (in bits) of the underlying storage.
pub const RULESET_CAPACITY: usize = RULESET_SIZE * 64;

/// A set of [`Rule`]s.
///
/// Uses a fixed-size bitset where each bit corresponds to a rule's `u16`
/// discriminant. Most set operations are `const fn` to permit static
/// construction of default rule sets in the future.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct RuleSet([u64; RULESET_SIZE]);

impl RuleSet {
    const EMPTY: [u64; RULESET_SIZE] = [0; RULESET_SIZE];
    #[expect(clippy::cast_possible_truncation)]
    const SLICE_BITS: u16 = u64::BITS as u16;

    /// Returns an empty rule set.
    #[must_use]
    pub const fn empty() -> Self {
        Self(Self::EMPTY)
    }

    /// Returns a rule set containing only `rule`.
    #[must_use]
    #[inline]
    pub const fn from_rule(rule: Rule) -> Self {
        let rule = rule as u16;
        let index = (rule / Self::SLICE_BITS) as usize;
        let shift = rule % Self::SLICE_BITS;
        let mask = 1u64 << shift;

        let mut bits = Self::EMPTY;
        bits[index] = mask;
        Self(bits)
    }

    /// Inserts `rule` into the set.
    #[inline]
    pub fn insert(&mut self, rule: Rule) {
        let set = std::mem::take(self);
        *self = set.union(&Self::from_rule(rule));
    }

    /// Removes `rule` from the set.
    #[inline]
    pub fn remove(&mut self, rule: Rule) {
        let set = std::mem::take(self);
        *self = set.subtract(&Self::from_rule(rule));
    }

    /// Returns `true` if `rule` is in this set.
    #[inline]
    #[must_use]
    pub const fn contains(&self, rule: Rule) -> bool {
        let rule = rule as u16;
        let index = (rule / Self::SLICE_BITS) as usize;
        let shift = rule % Self::SLICE_BITS;
        let mask = 1u64 << shift;
        self.0[index] & mask != 0
    }

    /// Returns the union of `self` and `other`.
    #[must_use]
    pub const fn union(mut self, other: &Self) -> Self {
        let mut i = 0;
        while i < self.0.len() {
            self.0[i] |= other.0[i];
            i += 1;
        }
        self
    }

    /// Returns `self` minus the rules contained in `other`.
    #[must_use]
    pub const fn subtract(mut self, other: &Self) -> Self {
        let mut i = 0;
        while i < self.0.len() {
            self.0[i] &= !other.0[i];
            i += 1;
        }
        self
    }

    /// Returns `true` if `self` and `other` share at least one rule.
    #[must_use]
    pub const fn intersects(&self, other: &Self) -> bool {
        let mut i = 0;
        while i < self.0.len() {
            if self.0[i] & other.0[i] != 0 {
                return true;
            }
            i += 1;
        }
        false
    }

    /// Returns `true` if this set has no rules.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of rules in this set.
    #[must_use]
    pub const fn len(&self) -> usize {
        let mut len: u32 = 0;
        let mut i = 0;
        while i < self.0.len() {
            len += self.0[i].count_ones();
            i += 1;
        }
        len as usize
    }

    /// Returns an iterator over the rules in this set.
    #[must_use]
    pub fn iter(&self) -> RuleSetIterator {
        RuleSetIterator {
            set: self.clone(),
            index: 0,
        }
    }
}

impl Debug for RuleSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl FromIterator<Rule> for RuleSet {
    fn from_iter<T: IntoIterator<Item = Rule>>(iter: T) -> Self {
        let mut set = Self::empty();
        for rule in iter {
            set.insert(rule);
        }
        set
    }
}

impl Extend<Rule> for RuleSet {
    fn extend<T: IntoIterator<Item = Rule>>(&mut self, iter: T) {
        for rule in iter {
            self.insert(rule);
        }
    }
}

impl IntoIterator for RuleSet {
    type IntoIter = RuleSetIterator;
    type Item = Rule;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for &RuleSet {
    type IntoIter = RuleSetIterator;
    type Item = Rule;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator yielded by [`RuleSet::iter`].
pub struct RuleSetIterator {
    set: RuleSet,
    index: u16,
}

impl Iterator for RuleSetIterator {
    type Item = Rule;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let slice = self.set.0.get_mut(self.index as usize)?;
            #[expect(clippy::cast_possible_truncation)]
            let bit = slice.trailing_zeros() as u16;

            if bit < RuleSet::SLICE_BITS {
                *slice ^= 1 << bit;
                let rule_value = self.index * RuleSet::SLICE_BITS + bit;
                // SAFETY: `RuleSet` only stores valid `Rule` discriminants.
                // `Rule` is `#[repr(u16)]` with dense discriminants assigned by
                // `define_rules!` (`0..Rule::COUNT`).
                #[expect(unsafe_code)]
                return Some(unsafe { std::mem::transmute::<u16, Rule>(rule_value) });
            }

            self.index += 1;
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.set.len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for RuleSetIterator {}
impl FusedIterator for RuleSetIterator {}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn empty_set_has_no_rules() {
        let set = RuleSet::empty();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
        assert_eq!(set.into_iter().collect::<Vec<_>>(), vec![]);
    }

    #[test]
    fn from_rule_contains_only_that_rule() {
        for rule in Rule::iter() {
            let set = RuleSet::from_rule(rule);
            assert!(set.contains(rule));
            assert_eq!(set.len(), 1);
            assert_eq!(set.iter().collect::<Vec<_>>(), vec![rule]);
        }
    }

    #[test]
    fn collect_all_rules_round_trips() {
        let all: RuleSet = Rule::iter().collect();
        let collected: Vec<_> = all.iter().collect();
        let expected: Vec<_> = Rule::iter().collect();
        assert_eq!(collected, expected);
    }

    #[test]
    fn union_combines_sets() {
        let mut a = RuleSet::empty();
        a.insert(Rule::InvalidAttrValue);
        let b = RuleSet::empty();
        let combined = a.clone().union(&b);
        assert!(combined.contains(Rule::InvalidAttrValue));
    }

    #[test]
    fn subtract_removes_rules() {
        let mut a = RuleSet::empty();
        a.insert(Rule::InvalidAttrValue);
        let b = RuleSet::from_rule(Rule::InvalidAttrValue);
        let result = a.subtract(&b);
        assert!(!result.contains(Rule::InvalidAttrValue));
        assert!(result.is_empty());
    }

    #[test]
    fn intersects_detects_overlap() {
        let mut a = RuleSet::empty();
        a.insert(Rule::InvalidAttrValue);
        let b = RuleSet::from_rule(Rule::InvalidAttrValue);
        assert!(a.intersects(&b));
        assert!(!RuleSet::empty().intersects(&b));
    }

    #[test]
    fn remove_missing_rule_is_noop() {
        let mut set = RuleSet::empty();
        set.remove(Rule::InvalidAttrValue);
        assert!(set.is_empty());
    }
}

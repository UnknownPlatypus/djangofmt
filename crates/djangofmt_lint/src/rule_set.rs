//! Bitset-backed set of enabled lint rules.
//!
//! Mirrors Ruff's `rule_set.rs` design as closely as is reasonable: the set is
//! backed by a `[u64; RULESET_SIZE]` array, membership is a branchless bit
//! test, and the set is built once via `FromIterator<Rule>`.
//!
//! # Deliberate divergence from Ruff
//! Ruff also has a `RuleTable { enabled, should_fix }` containing a second
//! bitset gating per-rule autofix application.  djangofmt does NOT gate fixes
//! per-rule (applicability is checked at apply time via `Applicability`), so
//! `RuleTable` / a `should_fix` bitset are intentionally absent.

use strum::EnumCount as _;

use crate::registry::Rule;

/// Number of `u64` words needed to hold one bit per `Rule` variant.
/// Computed from `Rule::COUNT` so it scales automatically as rules are added.
/// Written as `(N + 63) / 64` rather than `N.div_ceil(64)` to stay const-safe
/// on the MSRV (`div_ceil` is not yet const-stable).
#[allow(clippy::manual_div_ceil)]
const RULESET_SIZE: usize = (Rule::COUNT + 63) / 64;

/// Number of bits in each backing word.
#[allow(clippy::cast_possible_truncation)] // u64::BITS is 64, fits in u16
const SLICE_BITS: u16 = u64::BITS as u16;

/// A compact bitset storing the set of enabled lint rules.
///
/// Each rule occupies exactly one bit.  Membership testing is a single
/// array-index + shift, with no hashing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuleSet([u64; RULESET_SIZE]);

impl RuleSet {
    /// Return a `RuleSet` with a single rule set.
    #[must_use]
    #[inline]
    pub const fn from_rule(rule: Rule) -> Self {
        let mut set = Self([0u64; RULESET_SIZE]);
        let rule = rule as u16;
        let index = rule as usize / SLICE_BITS as usize;
        let shift = rule % SLICE_BITS;
        set.0[index] |= 1u64 << shift;
        set
    }

    /// Insert a rule into the set.
    #[inline]
    pub const fn insert(&mut self, rule: Rule) {
        let rule = rule as u16;
        let index = rule as usize / SLICE_BITS as usize;
        let shift = rule % SLICE_BITS;
        self.0[index] |= 1u64 << shift;
    }

    /// Return whether `rule` is in the set.
    #[must_use]
    #[inline]
    pub const fn contains(&self, rule: Rule) -> bool {
        let rule = rule as u16;
        let index = rule as usize / SLICE_BITS as usize;
        let shift = rule % SLICE_BITS;
        self.0[index] & (1u64 << shift) != 0
    }

    /// Merge another `RuleSet` into this one (union in place).
    #[inline]
    pub const fn union(&mut self, other: &Self) {
        let mut i = 0;
        while i < RULESET_SIZE {
            self.0[i] |= other.0[i];
            i += 1;
        }
    }

    /// Iterate over all rules in the set in ascending discriminant order.
    #[must_use]
    pub const fn iter(&self) -> RuleSetIterator {
        RuleSetIterator {
            set: *self,
            word_index: 0,
            word: self.0[0],
        }
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

/// Iterator over the rules in a [`RuleSet`].
///
/// Uses `trailing_zeros()` to find each set bit without examining zero words,
/// mirroring Ruff's iterator design.
pub struct RuleSetIterator {
    set: RuleSet,
    word_index: usize,
    word: u64,
}

impl Iterator for RuleSetIterator {
    type Item = Rule;

    fn next(&mut self) -> Option<Rule> {
        // Advance past any exhausted words.
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
        let global_index = self.word_index * SLICE_BITS as usize + bit as usize;
        #[allow(clippy::cast_possible_truncation)]
        Rule::from_repr(global_index as u16)
    }
}

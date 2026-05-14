use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumString};

use crate::rule_group::RuleGroup;
use crate::rules;
use crate::violation::Violation;

/// Functional categories for lint rules.
///
/// Categories help users enable/disable groups of related rules. Stability
/// (`Stable`/`Preview`/`Deprecated`/`Removed`) is orthogonal and lives on
/// [`RuleGroup`].
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    EnumIter,
    EnumString,
    strum::Display,
    Serialize,
    Deserialize,
)]
#[strum(serialize_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum RuleCategory {
    /// Errors that are likely to cause runtime crashes or logic failures.
    Correctness,
    /// Code that looks incorrect or useless.
    Suspicious,
    /// Code that violates stylistic conventions.
    Style,
    /// Code that is overly complex.
    Complexity,
}

/// The single source of truth for all lint rules.
///
/// # Format
/// ```text
/// (RuleName, ViolationStruct)
/// ```
///
/// # What this macro generates
/// - `Rule` enum with human-friendly kebab-case names (e.g., `invalid-attr-value`)
///   and an explicit `#[repr(u16)]` representation with dense, stable
///   discriminants (used by `RuleSet`).
/// - `Rule::category()` method returning the functional category (from `Violation::CATEGORY`)
/// - `Rule::group()` method returning the stability group (from `Violation::GROUP`)
/// - `Rule::COUNT` constant equal to the number of registered rules.
/// - Compile-time verification that each violation implements `Violation` with matching `RULE`
///   and that the total rule count fits in `RuleSet`.
///
/// # Adding a new rule
/// 1. Create the violation struct in `rules/` implementing `Violation`
///    (including `const GROUP: RuleGroup;`)
/// 2. Add entry here: `(RuleName, ViolationStruct)`
/// 3. Wire up the check in the appropriate `visit_*` method in `checker.rs`
///
/// The compiler will error if:
/// - The violation struct doesn't exist
/// - The violation doesn't implement `Violation`
/// - The violation's `RULE` doesn't match the rule name
/// - The total rule count exceeds `RULESET_CAPACITY`
macro_rules!
define_rules {
    (
        $(
            $(#[$meta:meta])*
            ($rule:ident, $violation:path)
        ),* $(,)?
    ) => {
        /// Unique identifier for every lint rule.
        ///
        /// Rule names are human-friendly kebab-case (e.g., `invalid-attr-value`).
        ///
        /// `#[repr(u16)]` is load-bearing: `RuleSet` indexes into a fixed-size
        /// bitset using the discriminant value, and `RuleSetIterator`
        /// transmutes back into `Rule` from that discriminant.
        #[repr(u16)]
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            EnumIter,
            EnumString,
            strum::Display,
            Serialize,
            Deserialize
        )]
        #[strum(serialize_all = "kebab-case")]
        #[serde(rename_all = "kebab-case")]
        pub enum Rule {
            $(
                $(#[$meta])*
                $rule,
            )*
        }

        impl Rule {
            /// Number of registered rules.
            pub const COUNT: usize = { 0_usize $( + { let _ = stringify!($rule); 1_usize } )* };

            /// Returns the functional category of the rule.
            #[must_use]
            pub const fn category(&self) -> RuleCategory {
                match self {
                    $( Rule::$rule => <$violation as Violation>::CATEGORY, )*
                }
            }

            /// Returns the stability group of the rule.
            #[must_use]
            pub const fn group(&self) -> RuleGroup {
                match self {
                    $( Rule::$rule => <$violation as Violation>::GROUP, )*
                }
            }
        }

        // Compile-time verification:
        // 1. The violation struct exists and implements Violation
        // 2. The violation's RULE matches this rule
        const _: () = {
            $(
                // This will fail if:
                // - $violation doesn't exist
                // - $violation doesn't implement Violation
                // - $violation::RULE != Rule::$rule
                const _: () = assert!(
                    matches!(
                        <$violation as Violation>::RULE,
                        Rule::$rule
                    ),
                    concat!(
                        "Violation ", stringify!($violation),
                        "::RULE must be Rule::", stringify!($rule)
                    )
                );
            )*
        };

        // Compile-time check that every registered rule fits in `RuleSet`.
        const _: () = assert!(
            Rule::COUNT <= $crate::rule_set::RULESET_CAPACITY,
            "RuleSet overflow: bump RULESET_SIZE in rule_set.rs",
        );
    };
}

// ============================================================================
// RULE DEFINITIONS
// ============================================================================
//
// Format: (RuleName, path::to::ViolationStruct)
//
// The rule name becomes a kebab-case string automatically via strum.
// Example: InvalidAttrValue -> "invalid-attr-value"

define_rules! {
    /// Validates attribute values against allowed values (e.g., `<form method>`).
    (InvalidAttrValue, rules::correctness::invalid_attr_value::InvalidAttrValue),
}

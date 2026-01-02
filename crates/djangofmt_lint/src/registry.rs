use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};

use crate::rules;
use crate::violation::Violation;

/// Functional categories for lint rules.
///
/// Categories help users enable/disable groups of related rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, EnumString, Serialize, Deserialize)]
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
    /// New rules that are not yet stable.
    Nursery,
}

/// The single source of truth for all lint rules.
///
/// # Format
/// ```text
/// (RuleName, category, ViolationStruct)
/// ```
///
/// # What this macro generates
/// - `Rule` enum with human-friendly kebab-case names (e.g., `invalid-attr-value`)
/// - `Rule::category()` method returning the functional category
/// - Compile-time verification that each violation implements `Violation` with matching `CODE`
///
/// # Adding a new rule
/// 1. Create the violation struct in `rules/` implementing `Violation`
/// 2. Add entry here: `(RuleName, Category, ViolationStruct)`
/// 3. Wire up the check in the appropriate `visit_*` method in `checker.rs`
///
/// The compiler will error if:
/// - The violation struct doesn't exist
/// - The violation doesn't implement `Violation`
/// - The violation's `CODE` doesn't match the rule name
macro_rules!
define_rules {
    (
        $(
            $(#[$meta:meta])*
            ($rule:ident, $category:ident, $violation:path)
        ),* $(,)?
    ) => {
        /// Unique identifier for every lint rule.
        ///
        /// Rule names are human-friendly kebab-case (e.g., `invalid-attr-value`).
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            EnumIter,
            EnumString,
            strum_macros::Display,
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
            /// Returns the functional category of the rule.
            #[must_use]
            pub const fn category(&self) -> RuleCategory {
                match self {
                    $( Rule::$rule => RuleCategory::$category, )*
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
    };
}

// ============================================================================
// RULE DEFINITIONS
// ============================================================================
//
// Format: (RuleName, Category, path::to::ViolationStruct)
//
// The rule name becomes a kebab-case string automatically via strum.
// Example: InvalidAttrValue -> "invalid-attr-value"

define_rules! {
    /// Validates attribute values against allowed values (e.g., `<form method>`).
    (InvalidAttrValue, Correctness, rules::correctness::invalid_attr_value::InvalidAttrValue),
}

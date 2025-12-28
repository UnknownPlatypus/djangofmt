use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};

use crate::rules;

/// Functional categories for lint rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, EnumString, Serialize, Deserialize)]
#[strum(serialize_all = "kebab-case")]
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
/// Format: `(EnumVariant, Category, StructName)`
///
/// The string representation of the rule code is automatically derived as kebab-case
/// of the enum variant name.
macro_rules! define_rules {
    (
        $(
            ($code:ident, $category:ident, $struct_name:ident)
        ),* $(,)?
    ) => {
        /// The unique identifier for every rule in the system.
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            strum_macros::EnumIter,
            strum_macros::EnumString,
            strum_macros::Display,
            serde::Serialize,
            serde::Deserialize
        )]
        #[strum(serialize_all = "kebab-case")]
        #[serde(rename_all = "kebab-case")]
        pub enum RuleCode {
            $( $code, )*
        }

        impl RuleCode {
            /// Returns the functional category of the rule.
            pub const fn category(&self) -> RuleCategory {
                match self {
                    $( RuleCode::$code => RuleCategory::$category, )*
                }
            }
        }

        // Compile-time check: Ensure the struct exists and implements Violation.
        #[allow(dead_code, non_snake_case)]
        fn _compile_time_check() {
            $(
                let _ = |_: rules::$struct_name| {};
            )*
        }
    };
}

// Define the rules here.
// We map the snake_case enum variant to kebab-case string automatically via strum.
define_rules! {
    (InvalidAttrValue, Correctness, InvalidAttrValue),
}

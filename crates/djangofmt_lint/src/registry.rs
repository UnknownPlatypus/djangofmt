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
    /// Code that is not accessible.
    /// See <https://developer.mozilla.org/en-US/docs/Learn_web_development/Core/Accessibility/HTML>
    Accessibility,
    /// New rules that are not yet stable.
    Nursery,
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
/// - `Rule::category()` method returning the functional category (from `Violation::CATEGORY`)
/// - Compile-time verification that each violation implements `Violation` with matching `CODE`
///
/// # Adding a new rule
/// 1. Create the violation struct in `rules/` implementing `Violation`
/// 2. Add entry here: `(RuleName, ViolationStruct)`
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
            ($rule:ident, $violation:path)
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
                    $( Rule::$rule => <$violation as Violation>::CATEGORY, )*
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
// Format: (RuleName, path::to::ViolationStruct)
//
// The rule name becomes a kebab-case string automatically via strum.
// Example: InvalidAttrValue -> "invalid-attr-value"

define_rules! {
    (InvalidAttrValue, rules::correctness::invalid_attr_value::InvalidAttrValue),
    (MissingImgAlt, rules::accessibility::missing_img_alt::MissingImgAlt),
    (MissingHtmlLang, rules::accessibility::missing_html_lang::MissingHtmlLang),
    (MissingImgDimensions, rules::accessibility::missing_img_dimensions::MissingImgDimensions),
    (JavascriptUrl, rules::suspicious::suspicious_url::JavascriptUrl),
    (UseHttps, rules::suspicious::suspicious_url::UseHttps),
    (UppercaseFormMethod, rules::style::attr_value_style::UppercaseFormMethod),
    (FormActionWhitespace, rules::style::attr_value_style::FormActionWhitespace),
    (EmptyAttrValue, rules::style::attr_value_style::EmptyAttrValue),
    (RedundantTypeAttr, rules::style::redundant_type_attr::RedundantTypeAttr),
}

use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumString};

use crate::fix::FixAvailability;
use crate::rules;
use crate::violation::{Violation, ViolationMetadata};

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
    /// Code that creates accessibility (a11y) barriers.
    Accessibility,
    /// New rules that are not yet stable.
    Nursery,
}

impl RuleCategory {
    /// Capitalized label for the docs table.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Correctness => "Correctness",
            Self::Suspicious => "Suspicious",
            Self::Style => "Style",
            Self::Complexity => "Complexity",
            Self::Accessibility => "Accessibility",
            Self::Nursery => "Nursery",
        }
    }
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
            /// Returns the functional category of the rule.
            #[must_use]
            pub const fn category(&self) -> RuleCategory {
                match self {
                    $( Rule::$rule => <$violation as Violation>::CATEGORY, )*
                }
            }

            /// Returns the rule's fix availability.
            #[must_use]
            pub const fn fix_availability(&self) -> FixAvailability {
                match self {
                    $( Rule::$rule => <$violation as Violation>::FIX_AVAILABILITY, )*
                }
            }

            /// Returns the rule's documentation, or [`None`] when the violation
            /// struct has no `///` doc comment.
            /// Captured by `#[derive(ViolationMetadata)]`.
            #[must_use]
            pub fn explanation(&self) -> Option<&'static str> {
                match self {
                    $( Rule::$rule => <$violation as ViolationMetadata>::explain(), )*
                }
            }

            /// Returns the source file of the violation struct as produced by `file!()` at the `#[derive(ViolationMetadata)]` site.
            /// Normally a workspace-root-relative path like `crates/djangofmt_lint/src/...`,
            /// but the exact form depends on build flags such as `--remap-path-prefix`.
            #[must_use]
            pub fn source_file(&self) -> &'static str {
                match self {
                    $( Rule::$rule => <$violation as ViolationMetadata>::file(), )*
                }
            }

            /// Returns the source line of the violation struct definition.
            #[must_use]
            pub fn source_line(&self) -> u32 {
                match self {
                    $( Rule::$rule => <$violation as ViolationMetadata>::line(), )*
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
    (UntrimmedBlocktranslate, rules::correctness::untrimmed_blocktranslate::UntrimmedBlocktranslate),
    (RedundantTypeAttr, rules::style::redundant_type_attr::RedundantTypeAttr),
    (JavascriptUrl, rules::suspicious::javascript_url::JavascriptUrl),
    (UppercaseFormMethod, rules::style::uppercase_form_method::UppercaseFormMethod),
    (MissingHtmlLang, rules::accessibility::missing_html_lang::MissingHtmlLang),
}

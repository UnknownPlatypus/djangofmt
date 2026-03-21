//! duplicate-attr: Detects duplicate HTML attributes on elements.
//!
//! ## Rules
//!
//! - **H037 / duplicate-attr**: Detects elements with duplicate native attribute names.
//!
//! ## Skipped cases
//!
//! - Jinja conditional attributes (`Attribute::JinjaBlock`) are not native attributes
//!   and are naturally skipped.

use rustc_hash::FxHashSet;

use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::Violation;

/// Violation for duplicate attribute names on an element.
///
/// Reports when the same attribute name appears more than once on an element.
#[derive(Debug, PartialEq, Eq)]
pub struct DuplicateAttr {
    pub name: String,
}

impl Violation for DuplicateAttr {
    const RULE: Rule = Rule::DuplicateAttr;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;

    fn message(&self) -> String {
        format!("Duplicate attribute '{}'.", self.name)
    }

    fn help(&self) -> Option<String> {
        Some("Remove the duplicate attribute.".to_string())
    }
}

/// Check elements for duplicate native attribute names.
pub fn check_duplicate_attr(element: &Element<'_>, checker: &mut Checker<'_>) {
    let mut seen: FxHashSet<String> = FxHashSet::default();

    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, .. }) = attr {
            let lower = name.to_ascii_lowercase();
            if seen.contains(&lower) {
                let offset = checker.source_offset(name);
                checker.report(
                    &DuplicateAttr {
                        name: (*name).to_string(),
                    },
                    (offset, name.len()).into(),
                );
            } else {
                seen.insert(lower);
            }
        }
    }
}

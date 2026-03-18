//! empty-tag: Detects empty HTML tag pairs that serve no purpose.
//!
//! ## Rules
//!
//! - **H020 / empty-tag-pair**: Flags elements with no children that aren't void or self-closing.
//!
//! ## Skipped cases
//!
//! - Void elements (e.g. `<br>`, `<hr>`, `<img>`) are naturally skipped.
//! - Self-closing elements are skipped.
//! - `<i>` and `<span>` elements that have a `class` attribute are skipped,
//!   as they are commonly used as icon placeholders (e.g. Font Awesome icons).

use markup_fmt::ast::{Attribute, Element};

use crate::Checker;
use crate::registry::Rule;
use crate::violation::Violation;

/// Violation for empty tag pairs.
///
/// Reports when a non-void, non-self-closing element has no children.
#[derive(Debug, PartialEq, Eq)]
pub struct EmptyTagPair {
    pub tag: String,
}

impl Violation for EmptyTagPair {
    const RULE: Rule = Rule::EmptyTagPair;

    fn message(&self) -> String {
        format!("Empty <{}> tag pair found. Consider removing.", self.tag)
    }

    fn help(&self) -> Option<String> {
        Some("Remove the empty tag pair or add content.".to_string())
    }
}

fn has_class_attr(element: &Element<'_>) -> bool {
    element.attrs.iter().any(|attr| {
        matches!(
            attr,
            Attribute::Native(native) if native.name.eq_ignore_ascii_case("class")
        )
    })
}

/// Check for empty tag pairs (non-void, non-self-closing elements with no children).
pub fn check(element: &Element<'_>, checker: &mut Checker<'_>) {
    if element.void_element || element.self_closing {
        return;
    }
    // Skip elements commonly used empty with a class (icons, styled slots)
    if (element.tag_name.eq_ignore_ascii_case("i") || element.tag_name.eq_ignore_ascii_case("span"))
        && has_class_attr(element)
    {
        return;
    }
    if element.children.is_empty() {
        let offset = checker.offset_of(element.tag_name);
        checker.report(
            &EmptyTagPair {
                tag: element.tag_name.to_string(),
            },
            (offset, element.tag_name.len()).into(),
        );
    }
}

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
//! - `<td>`, `<li>`, `<th>`, `<dt>`, `<dd>` are skipped (commonly empty in tables/lists).
//! - `<i>` and `<span>` elements that have a `class` attribute are skipped,
//!   as they are commonly used as icon placeholders (e.g. Font Awesome icons).

use markup_fmt::ast::{Attribute, Element};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
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
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;

    fn message(&self) -> String {
        format!("Empty <{}> tag pair found. Consider removing.", self.tag)
    }

    fn help(&self) -> Option<String> {
        Some("Remove the empty tag pair or add content.".to_string())
    }
}

/// Tags that are commonly empty and should not be flagged.
const fn is_excluded_tag(tag: &str) -> bool {
    tag.eq_ignore_ascii_case("td")
        || tag.eq_ignore_ascii_case("li")
        || tag.eq_ignore_ascii_case("th")
        || tag.eq_ignore_ascii_case("dt")
        || tag.eq_ignore_ascii_case("dd")
}

fn has_attr(element: &Element<'_>, attr_name: &str) -> bool {
    element.attrs.iter().any(|attr| {
        matches!(
            attr,
            Attribute::Native(native) if native.name.eq_ignore_ascii_case(attr_name)
        )
    })
}

/// Check for empty tag pairs (non-void, non-self-closing elements with no children).
pub fn check(element: &Element<'_>, checker: &mut Checker<'_>) {
    if element.void_element || element.self_closing {
        return;
    }

    if is_excluded_tag(element.tag_name) {
        return;
    }

    // Skip elements commonly used empty with a class (icons, styled slots)
    if (element.tag_name.eq_ignore_ascii_case("i") || element.tag_name.eq_ignore_ascii_case("span"))
        && has_attr(element, "class")
    {
        return;
    }

    // Skip <script> with src attribute (external scripts are legitimately empty)
    if element.tag_name.eq_ignore_ascii_case("script") && has_attr(element, "src") {
        return;
    }

    if element.children.is_empty() {
        let offset = checker.source_offset(element.tag_name);
        checker.report(
            &EmptyTagPair {
                tag: element.tag_name.to_string(),
            },
            (offset, element.tag_name.len()).into(),
        );
    }
}

//! avoid-element: Flags HTML elements that should be avoided in favour of better alternatives.
//!
//! ## Rules
//!
//! - **H036 / avoid-br-tag**: Flags use of `<br>` tags.

use markup_fmt::ast::Element;

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::Violation;

/// Violation for use of `<br>` tags.
///
/// Reports when a `<br>` tag is used. Block-level elements or CSS margins
/// are preferred over line-break tags.
#[derive(Debug, PartialEq, Eq)]
pub struct AvoidBrTag;

impl Violation for AvoidBrTag {
    const RULE: Rule = Rule::AvoidBrTag;
    const CATEGORY: RuleCategory = RuleCategory::Style;

    fn message(&self) -> String {
        "Avoid use of <br> tags.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Use block-level elements or CSS margins instead.".to_string())
    }
}

/// Check for `<br>` elements.
pub fn check_br(element: &Element<'_>, checker: &mut Checker<'_>) {
    if element.tag_name.eq_ignore_ascii_case("br") {
        let offset = checker.source_offset(element.tag_name);
        checker.report(&AvoidBrTag, (offset, element.tag_name.len()).into());
    }
}

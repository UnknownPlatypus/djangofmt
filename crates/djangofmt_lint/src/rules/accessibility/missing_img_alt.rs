use markup_fmt::ast::{Attribute, Element, Node};

use crate::registry::Rule;
use crate::violation::Violation;
use crate::{Checker, RuleCategory};

/// missing-img-alt: Img tag should have an alt attribute.
///
/// The `alt` attribute provides alternative text for screen readers and is
/// displayed when the image cannot be loaded. It is required for accessibility.
///
/// See <https://developer.mozilla.org/en-US/docs/Web/API/HTMLImageElement/alt>

#[derive(Debug, PartialEq, Eq)]
pub struct MissingImgAlt;

impl Violation for MissingImgAlt {
    const RULE: Rule = Rule::MissingImgAlt;
    const CATEGORY: RuleCategory = RuleCategory::Accessibility;

    fn message(&self) -> String {
        "Img tag should have an alt attribute.".to_string()
    }
}

/// Check that `<img>` elements have an `alt` attribute.
pub fn check(node: &Node<'_>, element: &Element<'_>, checker: &mut Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("img") {
        return;
    }

    let has_alt = element.attrs.iter().any(
        |attr| matches!(attr, Attribute::Native(native) if native.name.eq_ignore_ascii_case("alt")),
    );

    if !has_alt {
        let offset = checker.source_offset(node.raw);
        checker.report(&MissingImgAlt, (offset, node.raw.len()).into());
    }
}

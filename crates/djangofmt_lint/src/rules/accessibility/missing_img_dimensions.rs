use markup_fmt::ast::{Attribute, Element, Node};

use crate::registry::Rule;
use crate::violation::Violation;
use crate::{Checker, RuleCategory};

/// missing-img-dimensions: Img tag should have height and width attributes.
///
/// Explicit `height` and `width` on `<img>` prevent layout shifts (CLS) when the
/// image loads. Both attributes are needed so the browser can compute the aspect ratio.
///
/// See <https://developer.mozilla.org/en-US/docs/Learn_web_development/Core/Structuring_content/HTML_images>

#[derive(Debug, PartialEq, Eq)]
pub struct MissingImgDimensions;

impl Violation for MissingImgDimensions {
    const RULE: Rule = Rule::MissingImgDimensions;
    const CATEGORY: RuleCategory = RuleCategory::Accessibility;

    fn message(&self) -> String {
        "Img tag should have height and width attributes.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Add explicit height and width to avoid layout shifts.".to_string())
    }
}

/// Check that `<img>` elements have both `height` and `width` attributes.
pub fn check(node: &Node<'_>, element: &Element<'_>, checker: &mut Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("img") {
        return;
    }

    let has_height = element.attrs.iter().any(
        |attr| matches!(attr, Attribute::Native(native) if native.name.eq_ignore_ascii_case("height")),
    );

    let has_width = element.attrs.iter().any(
        |attr| matches!(attr, Attribute::Native(native) if native.name.eq_ignore_ascii_case("width")),
    );

    if !has_height || !has_width {
        let offset = checker.source_offset(node.raw);
        checker.report(&MissingImgDimensions, (offset, node.raw.len()).into());
    }
}

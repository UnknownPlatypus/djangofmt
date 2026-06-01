use markup_fmt::ast::Element;

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `<img>` tags that do not declare an `alt` attribute.
///
/// ## Why is this bad?
/// The `alt` attribute provides a textual alternative for screen readers and is rendered when the
/// image cannot be loaded. Without it, assistive technologies have no way to describe the image to
/// the user.
/// Decorative images should declare `alt=""` so screen readers skip them.
///
/// ## Example
/// ```html
/// <img src="photo.jpg">
/// ```
///
/// Use instead:
/// ```html
/// <img src="photo.jpg" alt="A photo of the team">
/// ```
///
/// ## References
/// - [MDN: HTML `alt` attribute](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/img#alt)
/// - [WCAG 1.1.1: Non-text Content](https://www.w3.org/WAI/WCAG21/Understanding/non-text-content.html)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct MissingImgAlt;

impl Violation for MissingImgAlt {
    const RULE: Rule = Rule::MissingImgAlt;
    const CATEGORY: RuleCategory = RuleCategory::Accessibility;

    #[derive_message_formats]
    fn message(&self) -> String {
        "`<img>` tag should declare an `alt` attribute.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Add `alt=\"\"` for decorative images, or a short description otherwise.".to_string())
    }
}

/// Reports the violation when an `<img>` tag is missing an `alt` attribute.
///
/// Driven by the centralized element dispatcher, which classifies the tag and
/// tracks `alt` presence (native or Jinja-declared) during its single attribute
/// pass, passing the result as `has_alt`.
pub fn report_if_missing(checker: &Checker<'_>, element: &Element<'_>, has_alt: bool) {
    if has_alt {
        return;
    }

    let offset = checker.source_offset(element.tag_name);
    checker.report_diagnostic(&MissingImgAlt, (offset, element.tag_name.len()).into());
}

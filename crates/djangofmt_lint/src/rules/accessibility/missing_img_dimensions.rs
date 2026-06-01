use markup_fmt::ast::Element;

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::declares_native_attr;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `<img>` tags that omit the `height` or `width` attribute.
///
/// ## Why is this bad?
/// Explicit `height` and `width` on `<img>` let the browser reserve space for the image before it
/// loads, preventing cumulative layout shift (CLS) as surrounding content jumps when the image
/// arrives. Both attributes are required, since the browser uses them together to derive the
/// intrinsic aspect ratio.
///
/// ## Example
/// ```html
/// <img src="photo.jpg" alt="photo">
/// ```
///
/// Use instead:
/// ```html
/// <img src="photo.jpg" alt="photo" height="100" width="200">
/// ```
///
/// ## References
/// - [MDN: `<img>` — Setting `width` and `height`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/img#setting_width_and_height)
/// - [web.dev: Optimize Cumulative Layout Shift](https://web.dev/articles/optimize-cls#images-without-dimensions)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct MissingImgDimensions;

impl Violation for MissingImgDimensions {
    const RULE: Rule = Rule::MissingImgDimensions;
    const CATEGORY: RuleCategory = RuleCategory::Accessibility;

    #[derive_message_formats]
    fn message(&self) -> String {
        "`<img>` tag should declare both `height` and `width` attributes.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some(
            "Add explicit `height` and `width` to avoid layout shifts as the image loads."
                .to_string(),
        )
    }
}

/// The caller guarantees `element` is an `<img>`.
pub fn check(element: &Element<'_>, checker: &Checker<'_>) {
    let mut has_height = false;
    let mut has_width = false;
    for attr in &element.attrs {
        has_height = has_height || declares_native_attr(attr, "height");
        has_width = has_width || declares_native_attr(attr, "width");
        if has_height && has_width {
            return;
        }
    }

    let offset = checker.source_offset(element.tag_name);
    checker.report_diagnostic(
        &MissingImgDimensions,
        (offset, element.tag_name.len()).into(),
    );
}

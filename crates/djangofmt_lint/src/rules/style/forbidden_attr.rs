//! forbidden-attr: Detects forbidden HTML attributes.
//!
//! ## Rules
//!
//! - **H021 / inline-style**: Detects elements with a `style` attribute.

use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::Rule;
use crate::violation::Violation;

/// Violation for elements with an inline `style` attribute.
///
/// Reports when any element has a `style` attribute, which should be replaced
/// with a CSS class.
#[derive(Debug, PartialEq, Eq)]
pub struct InlineStyle;

impl Violation for InlineStyle {
    const RULE: Rule = Rule::InlineStyle;

    fn message(&self) -> String {
        "Inline styles should be avoided.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Use a CSS class instead.".to_string())
    }
}

/// Check elements for inline `style` attributes.
pub fn check_inline_style(element: &Element<'_>, checker: &mut Checker<'_>) {
    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, .. }) = attr
            && name.eq_ignore_ascii_case("style")
        {
            let offset = checker.offset_of(name);
            checker.report(&InlineStyle, (offset, name.len()).into());
        }
    }
}

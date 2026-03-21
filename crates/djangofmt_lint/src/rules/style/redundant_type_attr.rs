//! redundant-type-attr: Detects redundant `type` attributes on `<script>` and `<style>` tags.
//!
//! ## Rules
//!
//! - **H024 / redundant-type-attr**: Detects `type="text/javascript"` on `<script>` and
//!   `type="text/css"` on `<style>` since these are the default values.

use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::Violation;

/// Violation for redundant `type` attribute on `<script>` or `<style>` tags.
///
/// Reports when `<script type="text/javascript">` or `<style type="text/css">` is used,
/// since these are the default values and the `type` attribute can be omitted.
#[derive(Debug, PartialEq, Eq)]
pub struct RedundantTypeAttr {
    pub tag: String,
    pub type_value: String,
}

impl Violation for RedundantTypeAttr {
    const RULE: Rule = Rule::RedundantTypeAttr;
    const CATEGORY: RuleCategory = RuleCategory::Style;

    fn message(&self) -> String {
        format!(
            "Redundant type=\"{}\" on <{}> tag.",
            self.type_value, self.tag
        )
    }

    fn help(&self) -> Option<String> {
        Some("The type attribute is unnecessary for the default script/style type.".to_string())
    }
}

/// Check `<script>` and `<style>` elements for redundant default `type` attributes.
pub fn check(element: &Element<'_>, checker: &mut Checker<'_>) {
    let tag = element.tag_name;

    let default_type = if tag.eq_ignore_ascii_case("script") {
        "text/javascript"
    } else if tag.eq_ignore_ascii_case("style") {
        "text/css"
    } else {
        return;
    };

    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value, .. }) = attr {
            if !name.eq_ignore_ascii_case("type") {
                continue;
            }

            let Some((value_str, offset)) = value else {
                continue;
            };

            if value_str.eq_ignore_ascii_case(default_type) {
                checker.report(
                    &RedundantTypeAttr {
                        tag: tag.to_string(),
                        type_value: (*value_str).to_string(),
                    },
                    (*offset, value_str.len()).into(),
                );
            }
        }
    }
}

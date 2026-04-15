use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::Violation;

/// ## What it does
/// Checks for redundant `type` attributes on `<script>` and `<style>` tags.
///
/// ## Why is this bad?
/// Since HTML5, `<script>` defaults to `type="text/javascript"` and `<style>` defaults to
/// `type="text/css"`. Specifying these default values is redundant and adds unnecessary noise.
///
/// Non-default types (e.g., `module`, `text/less`, `application/ld+json`) and template
/// interpolation are naturally excluded by exact-match comparison.
///
/// ## Example
/// ```html
/// <script type="text/javascript" src="app.js"></script>
/// <style type="text/css">.foo { color: red; }</style>
/// ```
///
/// Use instead:
/// ```html
/// <script src="app.js"></script>
/// <style>.foo { color: red; }</style>
/// ```
///
/// ## References
/// - [Google HTML/CSS Style Guide](https://google.github.io/styleguide/htmlcssguide.html#type_Attributes)
/// - [HTML spec: the script element](https://html.spec.whatwg.org/multipage/scripting.html#the-script-element)
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
        Some(format!(
            "Remove the `type` attribute. `<{}>` defaults to `type=\"{}\"`.",
            self.tag,
            self.type_value.to_ascii_lowercase()
        ))
    }
}

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

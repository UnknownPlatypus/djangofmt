use markup_fmt::ast::{Element, NativeAttribute};

use crate::Checker;
use crate::fix::FixAvailability;
use crate::fix::edits::delete_attr_fix;
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for redundant `type` attributes on `<script>` and `<style>` tags.
///
/// ## Why is this bad?
/// Since HTML5, `<script>` defaults to `type="text/javascript"` and `<style>` defaults to
/// `type="text/css"`. Specifying these default values is redundant and adds unnecessary noise.
///
/// Non-default types (e.g., `module`, `text/less`, `application/ld+json`) are excluded by
/// value comparison; values containing template interpolation are skipped explicitly.
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
/// ## Fix safety
/// This rule's fix is marked as safe: removing a `type` attribute whose value
/// matches the HTML5 default for the tag preserves runtime semantics.
///
/// ## References
/// - [Google HTML/CSS Style Guide](https://google.github.io/styleguide/htmlcssguide.html#type_Attributes)
/// - [HTML spec: the script element](https://html.spec.whatwg.org/multipage/scripting.html#the-script-element)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.9")]
pub struct RedundantTypeAttr {
    pub tag: String,
    pub type_value: String,
}

impl Violation for RedundantTypeAttr {
    const RULE: Rule = Rule::RedundantTypeAttr;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
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

    fn fix_title(&self) -> Option<String> {
        Some("Remove redundant `type` attribute".to_string())
    }
}

pub fn check(attr: &NativeAttribute<'_>, element: &Element<'_>, checker: &Checker<'_>) {
    let tag = element.tag_name;

    let default_type = if tag.eq_ignore_ascii_case("script") {
        "text/javascript"
    } else if tag.eq_ignore_ascii_case("style") {
        "text/css"
    } else {
        return;
    };

    let NativeAttribute {
        name,
        value: Some((value_str, offset)),
        quote,
    } = attr
    else {
        return;
    };

    if !name.eq_ignore_ascii_case("type") {
        return;
    }

    if contains_interpolation(value_str) {
        return;
    }

    if !value_str.eq_ignore_ascii_case(default_type) {
        return;
    }

    let mut guard = checker.report_diagnostic(
        &RedundantTypeAttr {
            tag: tag.to_string(),
            type_value: (*value_str).to_string(),
        },
        (*offset, value_str.len()).into(),
    );

    guard.set_fix(delete_attr_fix(
        checker.context(),
        name,
        value_str,
        *offset,
        quote.is_some(),
    ));
}

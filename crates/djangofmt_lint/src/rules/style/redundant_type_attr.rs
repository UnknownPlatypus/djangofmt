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

/// The HTML5 default `type` value for `<script>` / `<style>`, or `None` for any
/// other tag. The dispatcher only calls [`check_attr`] for these two tags.
pub const fn default_type_for(tag: &str) -> Option<&'static str> {
    if tag.eq_ignore_ascii_case("script") {
        Some("text/javascript")
    } else if tag.eq_ignore_ascii_case("style") {
        Some("text/css")
    } else {
        None
    }
}

/// Per-attribute check driven by the centralized element dispatcher.
///
/// The dispatcher pre-filters to a `type` attribute on a `<script>`/`<style>`
/// tag and supplies the matching `default_type` from [`default_type_for`].
pub fn check_attr(
    checker: &Checker<'_>,
    tag: &str,
    default_type: &str,
    name: &str,
    value_str: &str,
    offset: usize,
    quote: Option<char>,
) {
    if contains_interpolation(value_str) {
        return;
    }

    if !value_str.eq_ignore_ascii_case(default_type) {
        return;
    }

    let mut guard = checker.report_diagnostic(
        &RedundantTypeAttr {
            tag: tag.to_string(),
            type_value: value_str.to_string(),
        },
        (offset, value_str.len()).into(),
    );

    guard.set_fix(delete_attr_fix(
        checker.context(),
        name,
        value_str,
        offset,
        quote.is_some(),
    ));
}

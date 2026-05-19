use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::Violation;

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
#[derive(Debug, PartialEq, Eq)]
pub struct RedundantTypeAttr<'a> {
    pub tag: &'a str,
    pub type_value: &'a str,
}

impl Violation for RedundantTypeAttr<'_> {
    const RULE: Rule = Rule::RedundantTypeAttr;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

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

pub fn check(element: &Element<'_>, checker: &Checker<'_>) {
    let tag = element.tag_name;

    let default_type = if tag.eq_ignore_ascii_case("script") {
        "text/javascript"
    } else if tag.eq_ignore_ascii_case("style") {
        "text/css"
    } else {
        return;
    };

    for attr in &element.attrs {
        let Attribute::Native(NativeAttribute { name, value, quote }) = attr else {
            continue;
        };

        if !name.eq_ignore_ascii_case("type") {
            continue;
        }

        let Some((value_str, offset)) = value else {
            continue;
        };

        if contains_interpolation(value_str) {
            continue;
        }

        if !value_str.eq_ignore_ascii_case(default_type) {
            continue;
        }

        let mut guard = checker.report_diagnostic(
            &RedundantTypeAttr {
                tag,
                type_value: value_str,
            },
            (*offset, value_str.len()).into(),
        );

        let name_start = checker.source_offset(name);
        let attr_end = *offset + value_str.len() + usize::from(quote.is_some());

        // Absorb the whitespace separating this attribute from the previous
        // token so removing the only attribute leaves `<script>` rather than
        // `<script >`.
        let source = checker.context().source().as_bytes();
        let mut fix_start = name_start;
        while fix_start > 0 && source[fix_start - 1].is_ascii_whitespace() {
            fix_start -= 1;
        }

        guard.set_fix(Fix::safe_edit(Edit::deletion(
            (fix_start, attr_end - fix_start).into(),
        )));
    }
}

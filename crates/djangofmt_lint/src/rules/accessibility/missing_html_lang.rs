use markup_fmt::ast::{Attribute, Element, JinjaBlock, JinjaTagOrChildren};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `<html>` tags that do not declare a `lang` attribute.
///
/// ## Why is this bad?
/// The `lang` attribute on `<html>` declares the primary language of the document. Screen readers
/// use it to select the correct pronunciation rules, and search engines use it to index the page
/// for the right audience.
///
/// A `lang` attribute wrapped in a Jinja conditional (e.g. `{% if %}lang="en"{% endif %}`) is
/// treated as present, to avoid false positives on dynamic templates.
///
/// ## Example
/// ```html
/// <html>
/// </html>
/// ```
///
/// Use instead:
/// ```html
/// <html lang="en">
/// </html>
/// ```
///
/// ## References
/// - [MDN: HTML `lang` global attribute](https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/lang)
/// - [WCAG 3.1.1: Language of Page](https://www.w3.org/WAI/WCAG21/Understanding/language-of-page.html)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.9")]
pub struct MissingHtmlLang;

impl Violation for MissingHtmlLang {
    const RULE: Rule = Rule::MissingHtmlLang;
    const CATEGORY: RuleCategory = RuleCategory::Accessibility;

    #[derive_message_formats]
    fn message(&self) -> String {
        "`<html>` tag should declare a `lang` attribute.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Add `lang=\"en\"` (or the appropriate language code).".to_string())
    }
}

pub fn check(element: &Element<'_>, checker: &Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("html") {
        return;
    }

    if element.attrs.iter().any(attr_declares_lang) {
        return;
    }

    let offset = checker.source_offset(element.tag_name);
    checker.report_diagnostic(&MissingHtmlLang, (offset, element.tag_name.len()).into());
}

fn attr_declares_lang(attr: &Attribute<'_>) -> bool {
    match attr {
        Attribute::Native(native) => native.name.eq_ignore_ascii_case("lang"),
        Attribute::JinjaBlock(block) => jinja_block_declares_lang(block),
        _ => false,
    }
}

fn jinja_block_declares_lang(block: &JinjaBlock<'_, Attribute<'_>>) -> bool {
    block.body.iter().any(|item| match item {
        JinjaTagOrChildren::Children(children) => children.iter().any(attr_declares_lang),
        JinjaTagOrChildren::Tag(_) => false,
    })
}

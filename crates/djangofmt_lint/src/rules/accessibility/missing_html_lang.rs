use markup_fmt::ast::{Attribute, Element, Node};

use crate::registry::Rule;
use crate::violation::Violation;
use crate::{Checker, RuleCategory};

/// missing-html-lang: Html tag should have a lang attribute.
///
/// The `lang` attribute on `<html>` declares the document language, which helps
/// screen readers use the correct pronunciation and search engines index the page.
///
/// See <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/lang>

#[derive(Debug, PartialEq, Eq)]
pub struct MissingHtmlLang;

impl Violation for MissingHtmlLang {
    const RULE: Rule = Rule::MissingHtmlLang;
    const CATEGORY: RuleCategory = RuleCategory::Accessibility;

    fn message(&self) -> String {
        "Html tag should have a lang attribute.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Add lang=\"en\" or the appropriate language code.".to_string())
    }
}

/// Check that `<html>` elements have a `lang` attribute.
pub fn check(node: &Node<'_>, element: &Element<'_>, checker: &mut Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("html") {
        return;
    }

    let has_lang = element.attrs.iter().any(
        |attr| matches!(attr, Attribute::Native(native) if native.name.eq_ignore_ascii_case("lang")),
    );

    if !has_lang {
        let offset = checker.source_offset(node.raw);
        checker.report(&MissingHtmlLang, (offset, node.raw.len()).into());
    }
}

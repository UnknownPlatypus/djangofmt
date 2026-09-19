use std::borrow::Cow;

use markup_fmt::ast::{Element, NativeAttribute};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::declares_native_attr_matching;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `<html>` tags that do not declare a non-empty `lang` attribute.
///
/// ## Why is this bad?
/// The `lang` attribute on `<html>` declares the primary language of the document. Screen readers
/// use it to select the correct pronunciation rules, and search engines use it to index the page
/// for the right audience. An empty or valueless `lang` declares the language as unknown, which
/// is no better than omitting it.
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
    fn message(&self) -> Cow<'static, str> {
        "Missing or empty `lang` attribute on `<html>`".into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some("Add a `lang` attribute, e.g. `lang=\"en\"`".into())
    }
}

/// The caller guarantees `element` is an `<html>` element.
pub fn check(checker: &Checker<'_>, element: &Element<'_>) {
    if element
        .attrs
        .iter()
        .any(|attr| declares_native_attr_matching(attr, &declares_lang))
    {
        return;
    }

    checker.report_diagnostic(&MissingHtmlLang, checker.source_span(element.tag_name));
}

/// A `lang` with no value, or a blank one, names no language; a templated value counts.
const fn declares_lang(attr: &NativeAttribute<'_>) -> bool {
    attr.name.eq_ignore_ascii_case("lang")
        && matches!(attr.value, Some((value, _)) if !value.trim_ascii().is_empty())
}

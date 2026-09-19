use std::borrow::Cow;

use markup_fmt::ast::Element;

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::native_attrs;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

#[derive(Debug, PartialEq, Eq)]
pub enum LangViolation {
    /// No `lang` attribute is present on `<html>`.
    Absent,
    /// A `lang` attribute is present, but valueless or blank.
    Empty,
}

/// ## What it does
/// Checks for `<html>` tags that do not declare a non-empty `lang` attribute.
///
/// ## Why is this bad?
/// The `lang` attribute on `<html>` declares the primary language of the document.
/// Screen readers use it to select the correct pronunciation rules, and search engines use it to
/// index the page for the right audience.
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
pub struct MissingHtmlLang {
    pub kind: LangViolation,
}

impl Violation for MissingHtmlLang {
    const RULE: Rule = Rule::MissingHtmlLang;
    const CATEGORY: RuleCategory = RuleCategory::Accessibility;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        match self.kind {
            LangViolation::Absent => "Missing `lang` attribute on `<html>`".into(),
            LangViolation::Empty => "Empty `lang` attribute on `<html>`".into(),
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some(match self.kind {
            LangViolation::Absent => "Add a `lang` attribute, e.g. `lang=\"en\"`".into(),
            LangViolation::Empty => "Give `lang` a language tag, e.g. `lang=\"en\"`".into(),
        })
    }
}

/// The caller guarantees `element` is an `<html>` element.
pub fn check(checker: &Checker<'_>, element: &Element<'_>) {
    let mut kind = LangViolation::Absent;
    for lang in native_attrs(&element.attrs).filter(|attr| attr.name.eq_ignore_ascii_case("lang")) {
        match lang.value {
            // A value naming a language, templated or not, satisfies the rule.
            Some((value, _)) if !value.trim_ascii().is_empty() => return,
            _ => kind = LangViolation::Empty,
        }
    }

    checker.report_diagnostic(
        &MissingHtmlLang { kind },
        checker.source_span(element.tag_name),
    );
}

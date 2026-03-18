//! missing-attr: Rules that detect missing required attributes on HTML elements.
//!
//! ## Rules
//!
//! - **H013 / missing-img-alt**: Detects `<img>` tags without an `alt` attribute.
//! - **H005 / missing-html-lang**: Detects `<html>` tags without a `lang` attribute.
//! - **H006 / missing-img-dimensions**: Detects `<img>` tags missing `height` or `width`.
//!
//! ## Skipped cases
//!
//! - Elements with Jinja/Django block attributes are not inspected for dynamic attribute presence.

use markup_fmt::ast::{Attribute, Element};

use crate::Checker;
use crate::registry::Rule;
use crate::violation::Violation;

/// Violation for `<img>` tags missing an `alt` attribute.
#[derive(Debug, PartialEq, Eq)]
pub struct MissingImgAlt;

impl Violation for MissingImgAlt {
    const RULE: Rule = Rule::MissingImgAlt;

    fn message(&self) -> String {
        "img tag should have an alt attribute.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Add alt=\"description\" or alt=\"\" for decorative images.".to_string())
    }
}

/// Violation for `<html>` tags missing a `lang` attribute.
#[derive(Debug, PartialEq, Eq)]
pub struct MissingHtmlLang;

impl Violation for MissingHtmlLang {
    const RULE: Rule = Rule::MissingHtmlLang;

    fn message(&self) -> String {
        "html tag should have a lang attribute.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Add lang=\"en\" or the appropriate language code.".to_string())
    }
}

/// Violation for `<img>` tags missing `height` and/or `width` attributes.
#[derive(Debug, PartialEq, Eq)]
pub struct MissingImgDimensions;

impl Violation for MissingImgDimensions {
    const RULE: Rule = Rule::MissingImgDimensions;

    fn message(&self) -> String {
        "img tag should have height and width attributes.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Add explicit height and width to avoid layout shifts.".to_string())
    }
}

/// Check `<img>` tags for a missing `alt` attribute.
pub fn check_img_alt(element: &Element<'_>, checker: &mut Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("img") {
        return;
    }

    let has_alt = element.attrs.iter().any(|attr| {
        matches!(
            attr,
            Attribute::Native(native) if native.name.eq_ignore_ascii_case("alt")
        )
    });

    if !has_alt {
        let offset = checker.offset_of(element.tag_name);
        checker.report(&MissingImgAlt, (offset, element.tag_name.len()).into());
    }
}

/// Check `<html>` tags for a missing `lang` attribute.
pub fn check_html_lang(element: &Element<'_>, checker: &mut Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("html") {
        return;
    }

    let has_lang = element.attrs.iter().any(|attr| {
        matches!(
            attr,
            Attribute::Native(native) if native.name.eq_ignore_ascii_case("lang")
        )
    });

    if !has_lang {
        let offset = checker.offset_of(element.tag_name);
        checker.report(&MissingHtmlLang, (offset, element.tag_name.len()).into());
    }
}

/// Check `<img>` tags for missing `height` or `width` attributes.
pub fn check_img_dimensions(element: &Element<'_>, checker: &mut Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("img") {
        return;
    }

    let has_height = element.attrs.iter().any(|attr| {
        matches!(
            attr,
            Attribute::Native(native) if native.name.eq_ignore_ascii_case("height")
        )
    });

    let has_width = element.attrs.iter().any(|attr| {
        matches!(
            attr,
            Attribute::Native(native) if native.name.eq_ignore_ascii_case("width")
        )
    });

    if !has_height || !has_width {
        let offset = checker.offset_of(element.tag_name);
        checker.report(
            &MissingImgDimensions,
            (offset, element.tag_name.len()).into(),
        );
    }
}

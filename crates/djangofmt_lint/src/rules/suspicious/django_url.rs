//! django-url: Detects hardcoded Django URLs that should use template tags.
//!
//! ## Rules
//!
//! - **D004 / django-static-url**: Detects URL attributes with hardcoded `/static/` or `static/`
//!   paths that should use the `{% static %}` template tag.
//! - **D018 / django-url-pattern**: Detects hardcoded internal paths that should use the
//!   `{% url %}` template tag.
//!
//! ## Skipped cases
//!
//! - **Interpolation**: Values containing `{{` or `{%` are skipped (dynamic values).

use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::Violation;

/// Violation for hardcoded static URLs that should use `{% static %}`.
#[derive(Debug, PartialEq, Eq)]
pub struct DjangoStaticUrl;

impl Violation for DjangoStaticUrl {
    const RULE: Rule = Rule::DjangoStaticUrl;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;

    fn message(&self) -> String {
        "Static URL should use {% static %} template tag.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Use {% static 'path/to/file' %} instead of hardcoding '/static/...'.".to_string())
    }
}

/// Violation for hardcoded internal URLs that should use `{% url %}`.
#[derive(Debug, PartialEq, Eq)]
pub struct DjangoUrlPattern;

impl Violation for DjangoUrlPattern {
    const RULE: Rule = Rule::DjangoUrlPattern;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;

    fn message(&self) -> String {
        "Internal link should use {% url %} template tag.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Use {% url 'view_name' %} instead of hardcoding internal paths.".to_string())
    }
}

/// Check `<link>`, `<img>`, `<script>`, `<source>` elements for hardcoded static paths.
pub fn check_django_static_url(element: &Element<'_>, checker: &mut Checker<'_>) {
    let tag = element.tag_name;
    let is_static_tag = tag.eq_ignore_ascii_case("link")
        || tag.eq_ignore_ascii_case("img")
        || tag.eq_ignore_ascii_case("script")
        || tag.eq_ignore_ascii_case("source");

    if !is_static_tag {
        return;
    }

    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value, .. }) = attr {
            let is_url_attr = name.eq_ignore_ascii_case("href")
                || name.eq_ignore_ascii_case("src")
                || name.eq_ignore_ascii_case("srcset");

            if !is_url_attr {
                continue;
            }

            let Some((value_str, offset)) = value else {
                continue;
            };

            if contains_interpolation(value_str) {
                continue;
            }

            if value_str.starts_with("/static/") || value_str.starts_with("static/") {
                checker.report(&DjangoStaticUrl, (*offset, value_str.len()).into());
            }
        }
    }
}

/// Check URL attributes for hardcoded internal paths.
///
/// Checks `<a>` href, `<form>` action, and `<div/span/input>` data-url/data-src attributes.
pub fn check_django_url_pattern(element: &Element<'_>, checker: &mut Checker<'_>) {
    let tag = element.tag_name;

    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value, .. }) = attr {
            // Match element/attribute combinations per djLint D018 scope
            let is_url_attr = if tag.eq_ignore_ascii_case("a") {
                name.eq_ignore_ascii_case("href")
            } else if tag.eq_ignore_ascii_case("form") {
                name.eq_ignore_ascii_case("action")
            } else if tag.eq_ignore_ascii_case("div")
                || tag.eq_ignore_ascii_case("span")
                || tag.eq_ignore_ascii_case("input")
            {
                name.eq_ignore_ascii_case("data-url") || name.eq_ignore_ascii_case("data-src")
            } else {
                false
            };

            if !is_url_attr {
                continue;
            }

            let Some((value_str, offset)) = value else {
                continue;
            };

            if contains_interpolation(value_str) {
                continue;
            }

            if is_internal_path(value_str) {
                checker.report(&DjangoUrlPattern, (*offset, value_str.len()).into());
            }
        }
    }
}

/// Returns true if the value looks like a hardcoded internal path.
fn is_internal_path(value: &str) -> bool {
    // Must start with `/` or a word character to be considered an internal path
    let Some(first) = value.chars().next() else {
        return false;
    };

    if first != '/' && !first.is_ascii_alphanumeric() {
        return false;
    }

    let lower = value.to_ascii_lowercase();

    // Skip protocol URLs
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("javascript:")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
        || lower.starts_with("data:")
        || lower.starts_with("sms:")
    {
        return false;
    }

    // Skip anchors
    if value.starts_with('#') {
        return false;
    }

    // Skip protocol-relative URLs
    if value.starts_with("//") {
        return false;
    }

    // Skip static and media paths (handled by D004)
    if value.starts_with("/static/") || value.starts_with("/media/") {
        return false;
    }

    true
}

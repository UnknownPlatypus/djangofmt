//! django-url: Detects hardcoded Django URLs that should use template tags.
//!
//! ## Rules
//!
//! - **D004 / django-static-url**: Detects URL attributes with hardcoded `/static/` or `static/`
//!   paths that should use the `{% static %}` template tag.
//! - **D018 / django-url-pattern**: Detects `<a href>` with hardcoded internal paths that should
//!   use the `{% url %}` template tag.
//!
//! ## Skipped cases
//!
//! - **Interpolation**: Values containing `{{` or `{%` are skipped (dynamic values).

use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::Rule;
use crate::violation::Violation;

/// Violation for hardcoded static URLs that should use `{% static %}`.
///
/// Reports when URL attributes (`href`, `src`, `action`, `data`) contain values
/// starting with `/static/` or `static/`.
#[derive(Debug, PartialEq, Eq)]
pub struct DjangoStaticUrl;

impl Violation for DjangoStaticUrl {
    const RULE: Rule = Rule::DjangoStaticUrl;

    fn message(&self) -> String {
        "Static URL should use {% static %} template tag.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Use {% static 'path/to/file' %} instead of hardcoding '/static/...'.".to_string())
    }
}

/// Violation for hardcoded internal URLs in `<a href>` that should use `{% url %}`.
///
/// Reports when `<a href="...">` contains a hardcoded internal path starting with `/`,
/// excluding anchors, protocol-relative URLs, external URLs, static/media paths.
#[derive(Debug, PartialEq, Eq)]
pub struct DjangoUrlPattern;

impl Violation for DjangoUrlPattern {
    const RULE: Rule = Rule::DjangoUrlPattern;

    fn message(&self) -> String {
        "Internal link should use {% url %} template tag.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Use {% url 'view_name' %} instead of hardcoding internal paths.".to_string())
    }
}

/// Returns true if the value contains Jinja/Django interpolation markers.
#[inline]
fn contains_interpolation(value: &str) -> bool {
    value.contains("{{") || value.contains("{%")
}

/// Check URL attributes for hardcoded static paths.
pub fn check_django_static_url(element: &Element<'_>, checker: &mut Checker<'_>) {
    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value, .. }) = attr {
            let is_url_attr = name.eq_ignore_ascii_case("href")
                || name.eq_ignore_ascii_case("src")
                || name.eq_ignore_ascii_case("action")
                || name.eq_ignore_ascii_case("data");

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

/// Check `<a href>` attributes for hardcoded internal URLs.
pub fn check_django_url_pattern(element: &Element<'_>, checker: &mut Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("a") {
        return;
    }

    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value, .. }) = attr {
            if !name.eq_ignore_ascii_case("href") {
                continue;
            }

            let Some((value_str, offset)) = value else {
                continue;
            };

            if contains_interpolation(value_str) {
                continue;
            }

            // Must start with `/` to be an internal path
            if !value_str.starts_with('/') {
                continue;
            }

            // Skip anchors
            if value_str.starts_with('#') {
                continue;
            }

            // Skip protocol-relative URLs
            if value_str.starts_with("//") {
                continue;
            }

            // Skip external URLs (http/https)
            let lower = value_str.to_ascii_lowercase();
            if lower.starts_with("http://") || lower.starts_with("https://") {
                continue;
            }

            // Skip mailto:, tel:, data:
            if lower.starts_with("mailto:")
                || lower.starts_with("tel:")
                || lower.starts_with("data:")
            {
                continue;
            }

            // Skip /static/ and /media/ paths
            if value_str.starts_with("/static/") || value_str.starts_with("/media/") {
                continue;
            }

            checker.report(&DjangoUrlPattern, (*offset, value_str.len()).into());
        }
    }
}

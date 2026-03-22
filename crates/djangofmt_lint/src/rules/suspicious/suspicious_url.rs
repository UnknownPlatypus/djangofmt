//! suspicious-url: Detects suspicious URL patterns in HTML attributes.
//!
//! ## Rules
//!
//! - **H019 / javascript-url**: Detects `javascript:` URLs in `href` attributes.
//! - **H022 / use-https**: Detects `http://` URLs that should use `https://`.
//!
//! ## Skipped cases
//!
//! - **Interpolation**: Values containing `{{` or `{%` are skipped (dynamic values)

use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::Violation;

/// Violation for `javascript:` URLs in `href` attributes.
///
/// Reports when an `href` attribute value starts with `javascript:`.
#[derive(Debug, PartialEq, Eq)]
pub struct JavascriptUrl;

impl Violation for JavascriptUrl {
    const RULE: Rule = Rule::JavascriptUrl;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;

    fn message(&self) -> String {
        "Avoid 'javascript:' URLs.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Use an event handler and a real URL instead.".to_string())
    }
}

/// Violation for `http://` URLs that should use `https://`.
///
/// Reports when `href`, `src`, or `action` attribute values start with `http://`.
#[derive(Debug, PartialEq, Eq)]
pub struct UseHttps;

impl Violation for UseHttps {
    const RULE: Rule = Rule::UseHttps;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;

    fn message(&self) -> String {
        "Use HTTPS for external links.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Replace 'http://' with 'https://'.".to_string())
    }
}

/// Check an element's `href` attributes for `javascript:` URLs.
pub fn check_javascript_url(element: &Element<'_>, checker: &mut Checker<'_>) {
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

            if value_str
                .trim()
                .to_ascii_lowercase()
                .starts_with("javascript:")
            {
                checker.report(&JavascriptUrl, (*offset, value_str.len()).into());
            }
        }
    }
}

/// Check an element's `href`, `src`, and `action` attributes for `http://` URLs.
pub fn check_use_https(element: &Element<'_>, checker: &mut Checker<'_>) {
    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value, .. }) = attr {
            let is_url_attr = name.eq_ignore_ascii_case("href")
                || name.eq_ignore_ascii_case("src")
                || name.eq_ignore_ascii_case("action");

            if !is_url_attr {
                continue;
            }

            let Some((value_str, offset)) = value else {
                continue;
            };

            if contains_interpolation(value_str) {
                continue;
            }

            if value_str.starts_with("http://") {
                checker.report(&UseHttps, (*offset, value_str.len()).into());
            }
        }
    }
}

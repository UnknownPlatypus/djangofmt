use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::{contains_interpolation, srcset_candidates};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for asset URLs in `<link>`, `<img>`, `<script>`, and `<source>` elements that point at a
/// hardcoded `static/` path instead of going through Django's `{% static %}` template tag.
///
/// ## Why is this bad?
/// Hardcoding `/static/...` couples templates to a single value of Django's `STATIC_URL` setting.
/// Projects that serve assets from a CDN, version assets with `ManifestStaticFilesStorage`, or
/// mount static files under a different prefix end up with broken or unfingerprinted URLs. The
/// `{% static %}` tag resolves the configured storage backend at render time, so the same template
/// works across environments.
///
/// ## Example
/// ```html
/// <link rel="stylesheet" href="/static/css/app.css">
/// <img src="static/img/logo.png">
/// ```
///
/// Use instead:
/// ```html
/// {% load static %}
/// <link rel="stylesheet" href="{% static 'css/app.css' %}">
/// <img src="{% static 'img/logo.png' %}">
/// ```
///
/// ## References
/// - [Django: managing static files](https://docs.djangoproject.com/en/stable/howto/static-files/)
/// - [Django: the `{% static %}` template tag](https://docs.djangoproject.com/en/stable/ref/templates/builtins/#static)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct DjangoStaticUrl {
    pub attribute: &'static str,
}

impl Violation for DjangoStaticUrl {
    const RULE: Rule = Rule::DjangoStaticUrl;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;

    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Hardcoded static path in `{}`.", self.attribute)
    }

    fn help(&self) -> Option<String> {
        Some("Use `{% static 'path/to/file' %}` instead.".to_string())
    }
}

const URL_ATTRIBUTES: &[&str] = &["href", "src", "srcset"];

const fn is_asset_tag(tag_name: &str) -> bool {
    tag_name.eq_ignore_ascii_case("link")
        || tag_name.eq_ignore_ascii_case("img")
        || tag_name.eq_ignore_ascii_case("script")
        || tag_name.eq_ignore_ascii_case("source")
}

/// Returns `true` when `value` starts with `static/` or `/static/`.
fn starts_with_static_path(value: &str) -> bool {
    let rest = value.strip_prefix('/').unwrap_or(value);
    let Some(after) = rest.strip_prefix("static") else {
        return false;
    };
    // Require the segment boundary so we don't flag e.g. `staticpages/`.
    matches!(after.as_bytes().first(), Some(b'/'))
}

pub fn check(element: &Element<'_>, checker: &Checker<'_>) {
    if !is_asset_tag(element.tag_name) {
        return;
    }
    for attr in &element.attrs {
        let Attribute::Native(NativeAttribute {
            name,
            value: Some((value_str, offset)),
            ..
        }) = attr
        else {
            continue;
        };
        let Some(canonical) = URL_ATTRIBUTES
            .iter()
            .copied()
            .find(|candidate| candidate.eq_ignore_ascii_case(name))
        else {
            continue;
        };

        // `srcset` is a comma-separated candidate list; every other attribute
        // holds a single URL.
        if canonical == "srcset" {
            for (url, at) in srcset_candidates(value_str, *offset) {
                report_static_path(url, at, canonical, checker);
            }
        } else {
            report_static_path(value_str, *offset, canonical, checker);
        }
    }
}

/// Reports a URL that points at a hardcoded `static/` path.
/// `offset` is the byte offset of `url` in the source.
fn report_static_path(url: &str, offset: usize, attribute: &'static str, checker: &Checker<'_>) {
    if contains_interpolation(url) {
        return;
    }
    if starts_with_static_path(url) {
        checker.report_diagnostic(&DjangoStaticUrl { attribute }, (offset, url.len()).into());
    }
}

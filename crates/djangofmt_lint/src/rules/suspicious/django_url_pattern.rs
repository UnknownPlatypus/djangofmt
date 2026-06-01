use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for hardcoded internal URLs in HTML link attributes that should use Django's
/// `{% url %}` template tag.
///
/// ## Why is this bad?
/// Hardcoded internal paths duplicate the routing configuration into every template and break
/// silently when a `urls.py` entry is renamed or remounted. The `{% url %}` template tag resolves
/// paths from named URL patterns at render time, so the template stays correct across refactors
/// and respects URL namespacing (`app_name`, `instance_namespace`).
///
/// ## Example
/// ```html
/// <a href="/profile/">Profile</a>
/// <form action="/login/"></form>
/// ```
///
/// Use instead:
/// ```html
/// <a href="{% url 'profile' %}">Profile</a>
/// <form action="{% url 'login' %}"></form>
/// ```
///
/// ## References
/// - [Django: the `url` template tag](https://docs.djangoproject.com/en/stable/ref/templates/builtins/#url)
/// - [Django: URL dispatcher](https://docs.djangoproject.com/en/stable/topics/http/urls/)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(preview_since = "NEXT_DJANGOFMT_VERSION")]
pub struct DjangoUrlPattern {
    pub attribute: &'static str,
}

impl Violation for DjangoUrlPattern {
    const RULE: Rule = Rule::DjangoUrlPattern;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;

    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Hardcoded internal URL in `{}`.", self.attribute)
    }

    fn help(&self) -> Option<String> {
        Some("Use `{% url 'view_name' %}` to reference a named URL pattern.".to_string())
    }
}

const A_DIV_SPAN_INPUT_ATTRS: &[&str] = &["href", "data-url", "data-src", "action"];
const FORM_ATTRS: &[&str] = &["action"];

const fn url_attributes_for(tag_name: &str) -> &'static [&'static str] {
    if tag_name.eq_ignore_ascii_case("form") {
        return FORM_ATTRS;
    }
    if tag_name.eq_ignore_ascii_case("a")
        || tag_name.eq_ignore_ascii_case("div")
        || tag_name.eq_ignore_ascii_case("span")
        || tag_name.eq_ignore_ascii_case("input")
    {
        return A_DIV_SPAN_INPUT_ATTRS;
    }
    &[]
}

pub fn check(element: &Element<'_>, checker: &Checker<'_>) {
    let attr_names = url_attributes_for(element.tag_name);
    if attr_names.is_empty() {
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
        let Some(canonical) = attr_names
            .iter()
            .find(|candidate| candidate.eq_ignore_ascii_case(name))
        else {
            continue;
        };
        if contains_interpolation(value_str) {
            continue;
        }
        if is_hardcoded_internal_path(value_str) {
            checker.report_diagnostic(
                &DjangoUrlPattern {
                    attribute: canonical,
                },
                (*offset, value_str.len()).into(),
            );
        }
    }
}

/// Returns true if `value` looks like a hardcoded internal path that should use `{% url %}`.
///
/// Matches values whose first character is `/` (root-relative) or an ASCII word character
/// (relative), after excluding protocol-relative URLs, the bare site root, anything carrying a
/// URL scheme, file paths (`/favicon.ico`, `logo.png`), and bare hostnames (`www.example.com/…`).
fn is_hardcoded_internal_path(value: &str) -> bool {
    let Some(first) = value.chars().next() else {
        return false;
    };

    // Must start with `/` (root-relative) or a word character (relative).
    if first != '/' && !is_ascii_word_char(first) {
        return false;
    }

    // Protocol-relative URLs (`//cdn.example.com/...`).
    if value.starts_with("//") {
        return false;
    }

    // Site root: `/` alone is conventionally the home link and shouldn't be flagged.
    if value == "/" {
        return false;
    }

    // Any value carrying a URL scheme (`https:`, `mailto:`, `ftp:`, `tel:`, …) addresses an
    // external resource or a non-navigational target, not an internal route.
    if has_url_scheme(value) {
        return false;
    }

    // A value pointing at a file — its last path segment carries an extension (`/favicon.ico`,
    // `/assets/app.js`, `logo.png`) — is a static asset served via `{% static %}`, not a named
    // route. This holds for root-relative and relative paths alike.
    if last_segment_has_dot(value) {
        return false;
    }

    // A schemeless, relative value whose first segment is a hostname (`www.example.com/...`,
    // `example.com/path`) is an external link, not an internal route.
    if first != '/' && first_segment_has_dot(value) {
        return false;
    }

    true
}

const fn is_ascii_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Returns true if `value` begins with an RFC 3986 scheme, e.g. `https://…`, `mailto:…`, or
/// `ftp://…`. A scheme is `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )` followed by `:`.
fn has_url_scheme(value: &str) -> bool {
    let bytes = value.as_bytes();
    // A scheme must start with a letter.
    if !bytes.first().is_some_and(u8::is_ascii_alphabetic) {
        return false;
    }
    let scheme_len = bytes
        .iter()
        .take_while(|&&b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
        .count();
    bytes.get(scheme_len) == Some(&b':')
}

/// Returns true if the first path segment (up to the first `/`, `?`, or `#`) contains a `.` —
/// the shape of a hostname (`www.example.com`) or a file name (`logo.png`).
fn first_segment_has_dot(value: &str) -> bool {
    value
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(value)
        .contains('.')
}

/// Returns true if the last path segment (the text after the final `/`, with any query string or
/// fragment stripped) contains a `.` — the shape of a file name (`favicon.ico`, `app.js`).
fn last_segment_has_dot(value: &str) -> bool {
    value
        .split(['?', '#'])
        .next()
        .unwrap_or(value)
        .rsplit('/')
        .next()
        .unwrap_or(value)
        .contains('.')
}

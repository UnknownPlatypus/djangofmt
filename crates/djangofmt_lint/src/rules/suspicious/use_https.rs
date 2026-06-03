use std::net::IpAddr;

use markup_fmt::ast::NativeAttribute;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::srcset_candidates;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `http://` URLs in HTML attributes that load or link external resources.
///
/// ## Why is this bad?
/// `http://` traffic is unencrypted and can be intercepted or modified in transit.
/// Modern browsers also block mixed content (HTTP subresources on an HTTPS page),
/// so a single `http://` URL can silently break the page.
///
/// Prefer `https://` for all external links and subresources.
///
/// ## Example
/// ```html
/// <a href="http://example.com">Link</a>
/// ```
///
/// Use instead:
/// ```html
/// <a href="https://example.com">Link</a>
/// ```
///
/// ## Fix safety
/// This rule's fix is marked as unsafe: rewriting the scheme changes which endpoint the browser use.
/// The host may not serve HTTPS at all, so the fix can break a link or subresource that
/// previously worked over HTTP, and even when HTTPS is available it may serve different content .
///
/// ## References
/// - [MDN: Mixed content](https://developer.mozilla.org/en-US/docs/Web/Security/Mixed_content)
/// - [WHATWG Fetch: HTTPS state](https://fetch.spec.whatwg.org/#concept-request-https-state)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.10")]
pub struct UseHttps {
    pub attribute: &'static str,
}

impl Violation for UseHttps {
    const RULE: Rule = Rule::UseHttps;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Avoid `http://` URLs in `{}`.", self.attribute)
    }

    fn help(&self) -> Option<String> {
        Some("Use `https://` instead.".to_string())
    }

    fn fix_title(&self) -> Option<String> {
        Some("Replace `http://` with `https://`".to_string())
    }
}

const HTTP_SCHEME: &str = "http://";
const HTTPS_SCHEME: &str = "https://";

pub fn check(attr: &NativeAttribute<'_>, checker: &Checker<'_>) {
    let NativeAttribute {
        name,
        value: Some((value_str, offset)),
        ..
    } = attr
    else {
        return;
    };

    let Some(canonical) = canonical_url_attr(name) else {
        return;
    };

    // `srcset` is a comma-separated candidate list; every other attribute
    // holds a single URL.
    if canonical == "srcset" {
        for (url, at) in srcset_candidates(value_str, *offset) {
            report_http_scheme(url, at, canonical, checker);
        }
    } else {
        report_http_scheme(value_str, *offset, canonical, checker);
    }
}

/// The canonical name if `name` is a URL-bearing attribute, else `None`.
/// Matching on length first rejects non-URL attributes (the common case) cheaply.
fn canonical_url_attr(name: &str) -> Option<&'static str> {
    let candidates: &[&str] = match name.len() {
        3 => &["src", "url"],
        4 => &["href"],
        6 => &["action", "srcset"],
        8 => &["data-url"],
        _ => return None,
    };
    candidates
        .iter()
        .copied()
        .find(|c| c.eq_ignore_ascii_case(name))
}

/// Reports (and offers a fix for) a URL that uses the insecure `http://` scheme.
/// `offset` is the byte offset of `url` in the source.
fn report_http_scheme(url: &str, offset: usize, attribute: &'static str, checker: &Checker<'_>) {
    let trimmed = url.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let Some(rest) = trimmed.get(HTTP_SCHEME.len()..) else {
        return;
    };
    if !trimmed[..HTTP_SCHEME.len()].eq_ignore_ascii_case(HTTP_SCHEME) || is_local_host(rest) {
        return;
    }
    let scheme_span = (offset + url.len() - trimmed.len(), HTTP_SCHEME.len());
    let mut guard = checker.report_diagnostic(&UseHttps { attribute }, scheme_span.into());
    guard.set_fix(Fix::unsafe_edit(Edit::replacement(
        HTTPS_SCHEME,
        scheme_span.into(),
    )));
}

/// Whether the authority of a URL (the part after `http://`) refers to the local machine.
///
/// [spec]: https://w3c.github.io/webappsec-secure-contexts/#is-origin-trustworthy
fn is_local_host(after_scheme: &str) -> bool {
    // Authority is everything up to the path, query, or fragment.
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(after_scheme);
    // Drop any `user:pass@` userinfo prefix.
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    // Strip the port. A bracketed IPv6 literal (`[::1]:8080`) keeps everything
    // between the brackets; otherwise the host is everything before the colon.
    let host = host_port.strip_prefix('[').map_or_else(
        || host_port.split(':').next().unwrap_or(host_port),
        |rest| rest.split(']').next().unwrap_or(rest),
    );

    // `IpAddr::is_loopback` covers `127.0.0.0/8` and `::1` in every valid
    // spelling; fall back to the `localhost` name for non-IP hosts.
    if let Ok(ip) = host.parse::<IpAddr>() {
        return ip.is_loopback();
    }
    host.eq_ignore_ascii_case("localhost") || ends_with_ignore_ascii_case(host, ".localhost")
}

fn ends_with_ignore_ascii_case(haystack: &str, suffix: &str) -> bool {
    haystack.len() >= suffix.len()
        && haystack[haystack.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

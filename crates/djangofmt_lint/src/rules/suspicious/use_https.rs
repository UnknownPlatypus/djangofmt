use std::borrow::Cow;

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
/// Loopback, unspecified (`0.0.0.0`) and private-network addresses, and the `localhost`,
/// `.local` and `.test` names are exempt: no public certificate authority issues certificates
/// for them, so `https://` is not an option there.
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
/// This rule's fix is marked as unsafe: rewriting the scheme changes which endpoint the browser
/// requests. The host may not serve HTTPS at all, or may serve different content over it.
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
    fn message(&self) -> Cow<'static, str> {
        format!("Avoid `http://` URLs in `{}`", self.attribute).into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some("Use `https://` instead".into())
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Replace `http://` with `https://`")
    }
}

const HTTP_SCHEME: &str = "http://";
const HTTPS_SCHEME: &str = "https://";

pub fn check(checker: &Checker<'_>, attr: &NativeAttribute<'_>) {
    let NativeAttribute {
        name,
        value: Some((value_str, _)),
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
        for url in srcset_candidates(value_str) {
            report_http_scheme(checker, url, canonical);
        }
    } else {
        report_http_scheme(checker, value_str, canonical);
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
fn report_http_scheme(checker: &Checker<'_>, url: &str, attribute: &'static str) {
    let trimmed = url.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let Some((scheme, rest)) = trimmed.split_at_checked(HTTP_SCHEME.len()) else {
        return;
    };
    if !scheme.eq_ignore_ascii_case(HTTP_SCHEME) || is_local_host(rest) {
        return;
    }
    let scheme_span = checker.source_span(scheme);
    let mut guard = checker.report_diagnostic(&UseHttps { attribute }, scheme_span);
    guard.set_fix(Fix::unsafe_edit(Edit::replacement(
        HTTPS_SCHEME,
        scheme_span,
    )));
}

/// Whether the authority of a URL (the part after `http://`) refers to the local machine, a
/// private network, or a special-use name that no public certificate authority serves.
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

    // `is_loopback` covers `127.0.0.0/8` and `::1` in every valid spelling;
    // fall back to the reserved names for non-IP hosts.
    if let Ok(ip) = host.parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_unspecified(),
            IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unspecified(),
        };
    }
    host.eq_ignore_ascii_case("localhost")
        || [".localhost", ".local", ".test"]
            .iter()
            .any(|suffix| ends_with_ignore_ascii_case(host, suffix))
}

fn ends_with_ignore_ascii_case(haystack: &str, suffix: &str) -> bool {
    haystack.len() >= suffix.len()
        && haystack[haystack.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

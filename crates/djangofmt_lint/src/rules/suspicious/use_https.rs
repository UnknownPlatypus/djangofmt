use std::net::IpAddr;

use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
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
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
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

/// Attributes whose value is a URL subject to the HTTPS check.
const URL_ATTRIBUTES: &[&str] = &["href", "data-url", "action", "src", "url", "srcset"];

pub fn check(element: &Element<'_>, checker: &Checker<'_>) {
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
            .find(|candidate| candidate.eq_ignore_ascii_case(name))
        else {
            continue;
        };

        let trimmed = value_str.trim_start_matches(|c: char| c.is_ascii_whitespace());
        if trimmed
            .get(..HTTP_SCHEME.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(HTTP_SCHEME))
        {
            if is_local_host(&trimmed[HTTP_SCHEME.len()..]) {
                continue;
            }
            let leading_ws = value_str.len() - trimmed.len();
            let scheme_span = (*offset + leading_ws, HTTP_SCHEME.len());
            let mut guard = checker.report_diagnostic(
                &UseHttps {
                    attribute: canonical,
                },
                scheme_span.into(),
            );
            guard.set_fix(Fix::unsafe_edit(Edit::replacement(
                HTTPS_SCHEME,
                scheme_span.into(),
            )));
        }
    }
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

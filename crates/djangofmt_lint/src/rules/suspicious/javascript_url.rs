use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `javascript:` URLs in HTML elements.
///
/// ## Why is this bad?
/// `javascript:` URLs execute arbitrary code when the element is activated.
/// Any data interpolated into the URL becomes executable, which can allow cross-site scripting
/// (XSS) attacks. The pattern also bypasses Content Security Policy `script-src` directives.
/// Use a real URL and attach behavior with an event handler instead.
///
///
/// ## Example
/// ```html
/// <a href="javascript:alert('Hello, world!')">Click me</a>
/// ```
///
/// Use instead:
/// ```html
/// <button id="btn">Click me</button>
/// <script>
///   document.getElementById("btn").addEventListener("click", () => {
///     alert("Hello, world!");
///   });
/// </script>
/// ```
///
/// ## References
/// - [MDN: `javascript:` URLs](https://developer.mozilla.org/en-US/docs/Web/URI/Reference/Schemes/javascript)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.9")]
pub struct JavascriptUrl {
    pub attribute: &'static str,
}

impl Violation for JavascriptUrl {
    const RULE: Rule = Rule::JavascriptUrl;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;

    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Avoid `javascript:` URLs in `{}`.", self.attribute)
    }

    fn help(&self) -> Option<String> {
        Some("Use an event handler and a real URL instead.".to_string())
    }
}

const JS_SCHEME: &str = "javascript:";

/// The URL-bearing attribute names a `javascript:` URL is flagged in for the
/// given tag, or `&[]` for any other tag.
///
/// Exposed for the centralized element dispatcher so it can pre-classify a tag
/// once and dispatch matching attributes into [`check_attr`].
pub const fn url_attributes_for(tag_name: &str) -> &'static [&'static str] {
    if tag_name.eq_ignore_ascii_case("form") {
        return &["action"];
    }
    if tag_name.eq_ignore_ascii_case("a")
        || tag_name.eq_ignore_ascii_case("div")
        || tag_name.eq_ignore_ascii_case("span")
        || tag_name.eq_ignore_ascii_case("input")
    {
        return &["href", "data-url"];
    }
    &[]
}

/// Per-attribute check driven by the centralized element dispatcher.
///
/// `canonical` is the canonical attribute name (from [`url_attributes_for`]) the
/// dispatcher matched the attribute against.
pub fn check_attr(checker: &Checker<'_>, canonical: &'static str, value_str: &str, offset: usize) {
    if contains_interpolation(value_str) {
        return;
    }
    if value_str
        .trim_start()
        .get(..JS_SCHEME.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(JS_SCHEME))
    {
        checker.report_diagnostic(
            &JavascriptUrl {
                attribute: canonical,
            },
            (offset, value_str.len()).into(),
        );
    }
}

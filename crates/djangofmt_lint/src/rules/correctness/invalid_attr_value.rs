use std::borrow::Cow;

use markup_fmt::ast::{Element, NativeAttribute};

use crate::Checker;
use crate::html_spec::{self, is_keyword};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::{closest_match, contains_interpolation};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// The most keywords the help lists, so it stays under a dozen words.
const MAX_LISTED_KEYWORDS: usize = 8;

/// ## What it does
/// Checks for HTML attributes whose value is not one of the keywords the HTML specification allows,
/// such as `<form method>`, `<input type>` or `<img loading>`.
///
/// ## Why is this bad?
/// Browsers silently ignore an unknown keyword and fall back to a default, which usually does not
/// match the author's intent: `<form method="put">` sends a `GET` request and
/// `<img loading="lazzy">` loads eagerly. The resulting bug is easy to miss because the page still
/// renders.
///
/// Keywords match case-insensitively. Values containing template interpolation (`{{ ... }}` or
/// `{% ... %}`), custom elements, and attributes whose keywords depend on another attribute, such
/// as `<link as>`, are skipped.
///
/// ## Example
///
/// ```html
/// <form method="put"></form>
/// ```
///
/// Use instead:
///
/// ```html
/// <form method="post"></form>
/// ```
///
/// ## References
/// - [HTML Living Standard: Keywords and enumerated attributes](https://html.spec.whatwg.org/multipage/common-microsyntaxes.html#keywords-and-enumerated-attributes)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.5")]
pub struct InvalidAttrValue {
    pub value: String,
    pub attribute: String,
    pub allowed: &'static [&'static str],
    pub suggestion: Option<&'static str>,
}

impl Violation for InvalidAttrValue {
    const RULE: Rule = Rule::InvalidAttrValue;
    const CATEGORY: RuleCategory = RuleCategory::Correctness;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        format!(
            "Invalid value `{}` for attribute `{}`",
            self.value, self.attribute,
        )
        .into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        if let Some(keyword) = self.suggestion {
            return Some(format!("Did you mean `{keyword}`?").into());
        }
        if self.allowed.len() > MAX_LISTED_KEYWORDS {
            return None;
        }
        let allowed: Vec<_> = self
            .allowed
            .iter()
            .map(|&keyword| if keyword.is_empty() { "\"\"" } else { keyword })
            .collect();
        Some(format!("Use one of: {}", allowed.join(", ")).into())
    }
}

/// Check a single attribute for a value outside its keywords.
pub fn check(checker: &Checker<'_>, attr: &NativeAttribute<'_>, element: &Element<'_>) {
    let NativeAttribute {
        name,
        value: Some((value, _)),
        ..
    } = attr
    else {
        return;
    };
    let Some(keywords) = html_spec::keywords(element.tag_name, name) else {
        return;
    };
    if is_keyword(keywords, value) || contains_interpolation(value) {
        return;
    }

    checker.report_diagnostic(
        &InvalidAttrValue {
            value: (*value).into(),
            attribute: (*name).into(),
            allowed: keywords,
            suggestion: closest_match(&value.to_ascii_lowercase(), keywords.iter().copied()),
        },
        checker.source_span(value),
    );
}

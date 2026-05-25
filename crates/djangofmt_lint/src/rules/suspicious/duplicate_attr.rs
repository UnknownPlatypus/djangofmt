use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for the same attribute name appearing more than once on an HTML element.
///
/// ## Why is this bad?
/// HTML defines attribute names as case-insensitive and an element may contain a given attribute
/// at most once. When duplicates are present, browsers keep the first occurrence and silently
/// discard the rest, which usually does not match the author's intent.
///
/// Attribute names are compared case-insensitively against an exact match — `width` and
/// `stroke-width` are different attributes and do not collide. Any attribute wrapped in a
/// Jinja block (e.g. `{% if %}...{% endif %}`, `{% for %}...{% endfor %}`) is skipped, since
/// the attribute is not unconditionally present on the element. Duplicates within a single
/// block branch are likewise not flagged today.
///
/// ## Example
/// ```html
/// <br class="a" id="asdf" class="b" />
/// ```
///
/// Use instead:
/// ```html
/// <br class="a b" id="asdf" />
/// ```
///
/// ## References
/// - [HTML spec: attributes](https://html.spec.whatwg.org/multipage/syntax.html#attributes-2)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct DuplicateAttr<'a> {
    pub name: &'a str,
}

impl Violation for DuplicateAttr<'_> {
    const RULE: Rule = Rule::DuplicateAttr;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;

    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Duplicate attribute `{}`.", self.name)
    }

    fn help(&self) -> Option<String> {
        Some(format!(
            "Remove the duplicate `{}` attribute, or merge its value into the first occurrence (browsers keep the first one).",
            self.name
        ))
    }
}

pub fn check(element: &Element<'_>, checker: &Checker<'_>) {
    // Fast path: with fewer than 2 attributes there can be no duplicates.
    if element.attrs.len() < 2 {
        return;
    }

    for (i, attr) in element.attrs.iter().enumerate() {
        let Attribute::Native(NativeAttribute { name, .. }) = attr else {
            continue;
        };

        let is_duplicate = element.attrs[..i].iter().any(|prior| {
            matches!(
                prior,
                Attribute::Native(NativeAttribute { name: prior_name, .. })
                    if prior_name.eq_ignore_ascii_case(name)
            )
        });

        if is_duplicate {
            let offset = checker.source_offset(name);
            checker.report_diagnostic(&DuplicateAttr { name }, (offset, name.len()).into());
        }
    }
}

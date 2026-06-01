use markup_fmt::ast::{Attribute, NativeAttribute};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for the same attribute name appearing more than once on an HTML element.
///
/// ## Why is this bad?
/// HTML defines attribute names as case-insensitive and an element may contain a given attribute at most once.
/// When duplicates are present, browsers keep the first occurrence and silently discard the rest,
/// which usually does not match the author's intent.
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
#[violation_metadata(stable_since = "0.2.9")]
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

/// Per-attribute check driven by the centralized element dispatcher.
///
/// `prior_attrs` is the slice of attributes that precede the current one
/// (`element.attrs[..i]`); `name` is the current native attribute's name. Reports
/// the current attribute as a duplicate when a prior native attribute shares its
/// (case-insensitive) name, matching the original `element.attrs[..i]` scan so
/// the first occurrence is kept and each later one flagged.
pub fn check_attr(checker: &Checker<'_>, prior_attrs: &[Attribute<'_>], name: &str) {
    let is_duplicate = prior_attrs.iter().any(|prior| {
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

use std::borrow::Cow;

use markup_fmt::ast::{Attribute, Element, JinjaTagOrChildren, NativeAttribute};
use smallvec::SmallVec;

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
    fn message(&self) -> Cow<'static, str> {
        format!("Duplicate attribute `{}`", self.name).into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some(format!("Remove the duplicate `{}` attribute", self.name).into())
    }
}

pub fn check(checker: &Checker<'_>, element: &Element<'_>) {
    // Fast path: a lone attribute collides with nothing, unless it is a block holding several.
    if element.attrs.len() < 2 && !matches!(element.attrs.first(), Some(Attribute::JinjaBlock(_))) {
        return;
    }

    let mut seen = SmallVec::<[(&str, usize); 8]>::new();
    visit(checker, &element.attrs, 0, &mut 0, &mut seen);
}

/// Scope 0 is the element itself; every branch of a Jinja block gets its own id, so an attribute
/// collides with the element's own attributes and with its branch, never with a sibling branch.
fn visit<'s>(
    checker: &Checker<'_>,
    attrs: &[Attribute<'s>],
    scope: usize,
    next_scope: &mut usize,
    seen: &mut SmallVec<[(&'s str, usize); 8]>,
) {
    for attr in attrs {
        match attr {
            Attribute::Native(NativeAttribute { name, .. }) => {
                let is_duplicate = seen.iter().any(|&(prior, prior_scope)| {
                    prior.eq_ignore_ascii_case(name)
                        && (prior_scope == 0 || scope == 0 || prior_scope == scope)
                });
                if is_duplicate {
                    checker.report_diagnostic(&DuplicateAttr { name }, checker.source_span(name));
                }
                seen.push((name, scope));
            }
            Attribute::JinjaBlock(block) => {
                for item in &block.body {
                    if let JinjaTagOrChildren::Children(children) = item {
                        *next_scope += 1;
                        visit(checker, children, *next_scope, next_scope, seen);
                    }
                }
            }
            _ => {}
        }
    }
}

use markup_fmt::ast::{Element, Node, NodeKind};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for non-void elements with no attributes whose open and close tags wrap no content.
///
/// ## Why is this bad?
/// A bare `<tag></tag>` pair renders nothing, so it is often leftover scaffolding or a typo.
///
/// The HTML specification recommends, "as a general rule," that such elements contain at least
/// one node of palpable content. That is explicitly *not* a hard requirement — an element may
/// be empty legitimately "when it is used as a placeholder which will later be filled in by a
/// script, or when the element is part of a template".
///
/// ## Example
/// ```html
/// <div>Welcome<span></span></div>
/// ```
///
/// Use instead:
/// ```html
/// <div>Welcome</div>
/// ```
///
/// ## References
/// - [HTML spec: void elements](https://html.spec.whatwg.org/multipage/syntax.html#void-elements)
/// - [HTML spec: palpable content](https://html.spec.whatwg.org/multipage/dom.html#palpable-content)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(preview_since = "NEXT_DJANGOFMT_VERSION")]
pub struct EmptyTagPair {
    pub tag: String,
}

impl Violation for EmptyTagPair {
    const RULE: Rule = Rule::EmptyTagPair;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;

    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Empty `<{}>` tag pair.", self.tag)
    }

    fn help(&self) -> Option<String> {
        Some("Remove the empty tag pair or add content.".to_string())
    }
}

/// Tags whose empty form is legitimate rather than suspicious.
///
/// - `td`, `th`, `li`, `dt`, `dd`: kept empty to preserve table or list structure.
/// - `textarea`, `select`, `output`, `option`: form controls whose empty or script-populated
///   state is their normal initial state (`<option></option>` is a common blank placeholder).
/// - `canvas`: a script-rendered drawing surface whose children are fallback content only.
/// - `slot`: the default slot of a web component.
/// - `pre`:  a whitespace-only `<pre>` renders meaningful content and is not "empty".
const EXCLUDED_TAGS: &[&str] = &[
    "td", "th", "li", "dt", "dd", "textarea", "select", "output", "option", "canvas", "slot", "pre",
];

fn is_excluded_tag(tag: &str) -> bool {
    EXCLUDED_TAGS
        .iter()
        .any(|excluded| tag.eq_ignore_ascii_case(excluded))
}

/// Returns `true` when `children` is either empty or contains only whitespace-only text nodes.
fn has_only_whitespace(children: &[Node<'_>]) -> bool {
    children.iter().all(|child| match &child.kind {
        NodeKind::Text(text) => text.raw.chars().all(char::is_whitespace),
        _ => false,
    })
}

pub fn check(element: &Element<'_>, checker: &Checker<'_>) {
    if element.void_element
        || element.self_closing
        || !element.attrs.is_empty()
        || is_excluded_tag(element.tag_name)
        || !has_only_whitespace(&element.children)
    {
        return;
    }

    let offset = checker.source_offset(element.tag_name);
    checker.report_diagnostic(
        &EmptyTagPair {
            tag: element.tag_name.to_string(),
        },
        (offset, element.tag_name.len()).into(),
    );
}

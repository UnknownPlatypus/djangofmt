use markup_fmt::ast::{Element, Node, NodeKind};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for non-void elements with no attributes whose open and close tags wrap no content.
///
/// ## Why is this bad?
/// A bare `<tag></tag>` pair renders nothing and carries no metadata, so it is almost always
/// either leftover scaffolding or a typo. Remove it (or replace it with a self-closing element
/// where appropriate) to keep the template intentional and easier to scan.
///
/// Whitespace-only content (newlines, indentation) is treated as empty, matching the way the
/// browser would render the element. Cells used to preserve table or list structure
/// (`<td>`, `<li>`, `<th>`, `<dt>`, `<dd>`) are exempt.
///
/// ## Example
/// ```html
/// <div></div>
/// <span>
/// </span>
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
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
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

/// Tags commonly left empty to preserve table or list structure.
const EXCLUDED_TAGS: &[&str] = &["td", "li", "th", "dt", "dd"];

fn is_excluded_tag(tag: &str) -> bool {
    EXCLUDED_TAGS
        .iter()
        .any(|excluded| tag.eq_ignore_ascii_case(excluded))
}

/// Returns `true` when `children` is either empty or contains only whitespace-only text nodes.
///
/// Any non-text child (nested element, Jinja block, interpolation, comment, ...) makes the
/// element dynamically-populated and disqualifies it from the rule.
fn has_only_whitespace(children: &[Node<'_>]) -> bool {
    children.iter().all(|child| match &child.kind {
        NodeKind::Text(text) => text.raw.chars().all(char::is_whitespace),
        _ => false,
    })
}

pub fn check(element: &Element<'_>, checker: &Checker<'_>) {
    if element.void_element || element.self_closing {
        return;
    }

    if !element.attrs.is_empty() {
        return;
    }

    if is_excluded_tag(element.tag_name) {
        return;
    }

    if !has_only_whitespace(&element.children) {
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

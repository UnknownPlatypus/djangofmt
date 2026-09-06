use std::borrow::Cow;

use markup_fmt::ast::{JinjaBlock, JinjaTagOrChildren, Node, NodeKind, Root};
use markup_fmt::parser::parse_jinja_tag_name;

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for HTML documents that contain an `<html>` tag but no `<!DOCTYPE html>` declaration.
///
/// ## Why is this bad?
/// HTML5 requires a DOCTYPE declaration at the top of every document. Without it, browsers fall
/// back to "quirks mode", which emulates legacy rendering bugs and applies different CSS box-model
/// rules. The result is inconsistent layout across browsers and behaviour that is hard to debug.
///
/// Template partials (files with a root-level `{% extends %}` tag or `{% block %}` block) are
/// assumed to inherit the DOCTYPE from their parent template and are not flagged.
///
/// ## Example
/// ```html
/// <html lang="en">
///   <head><title>Page</title></head>
///   <body>Content</body>
/// </html>
/// ```
///
/// Use instead:
/// ```html
/// <!DOCTYPE html>
/// <html lang="en">
///   <head><title>Page</title></head>
///   <body>Content</body>
/// </html>
/// ```
///
/// ## References
/// - [HTML spec: The DOCTYPE](https://html.spec.whatwg.org/multipage/syntax.html#the-doctype)
/// - [MDN: Doctype](https://developer.mozilla.org/en-US/docs/Glossary/Doctype)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.9")]
pub struct MissingDoctype;

impl Violation for MissingDoctype {
    const RULE: Rule = Rule::MissingDoctype;
    const CATEGORY: RuleCategory = RuleCategory::Style;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        "Missing `<!DOCTYPE html>` declaration".into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some("Add `<!DOCTYPE html>` before the `<html>` tag".into())
    }
}

pub fn check(root: &Root<'_>, checker: &Checker<'_>) {
    let mut html_element = None;
    let mut has_doctype = false;

    for node in &root.children {
        match &node.kind {
            NodeKind::JinjaBlock(block) if is_block_partial(block) => return,
            NodeKind::JinjaTag(tag) if parse_jinja_tag_name(tag) == "extends" => return,
            NodeKind::Doctype(_) => has_doctype = true,
            NodeKind::Element(el)
                if html_element.is_none() && el.tag_name.eq_ignore_ascii_case("html") =>
            {
                html_element = Some(el);
            }
            _ => {}
        }
    }

    if has_doctype {
        return;
    }

    let Some(html) = html_element else {
        return;
    };

    checker.report_diagnostic(&MissingDoctype, checker.source_span(html.tag_name));
}

/// Returns `true` if the block opens with `{% block %}`, marking the file as a partial.
/// Other root-level blocks (`{% if %}`, `{% for %}`, ...) are legitimate in full documents.
fn is_block_partial(block: &JinjaBlock<'_, Node<'_>>) -> bool {
    matches!(
        block.body.first(),
        Some(JinjaTagOrChildren::Tag(tag)) if parse_jinja_tag_name(tag) == "block"
    )
}

use std::borrow::Cow;

use markup_fmt::ast::{Element, JinjaBlock, JinjaTagOrChildren, Node, NodeKind, Root};
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
/// The declaration must come before the `<html>` tag: one placed after it still leaves the
/// browser in quirks mode. A DOCTYPE or `<html>` tag written inside a `{% if %}` or `{% for %}`
/// block counts like one written at the top level.
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

pub fn check(checker: &Checker<'_>, root: &Root<'_>) {
    if let Scan::Html(html) = scan(&root.children, &mut false) {
        checker.report_diagnostic(&MissingDoctype, checker.source_span(html.tag_name));
    }
}

/// Outcome of walking the document in source order.
enum Scan<'a, 's> {
    /// Nothing decisive yet.
    Continue,
    /// A partial, or an `<html>` preceded by a DOCTYPE: nothing to report.
    Done,
    /// The first `<html>`, reached before any DOCTYPE.
    Html(&'a Element<'s>),
}

/// Walks `nodes` in source order, descending into Jinja blocks, until an `<html>` tag or a
/// partial marker settles the outcome.
fn scan<'a, 's>(nodes: &'a [Node<'s>], doctype_seen: &mut bool) -> Scan<'a, 's> {
    for node in nodes {
        match &node.kind {
            NodeKind::JinjaTag(tag) if parse_jinja_tag_name(tag) == "extends" => return Scan::Done,
            NodeKind::JinjaBlock(block) if is_block_partial(block) => return Scan::Done,
            NodeKind::JinjaBlock(block) => {
                for item in &block.body {
                    if let JinjaTagOrChildren::Children(children) = item {
                        match scan(children, doctype_seen) {
                            Scan::Continue => {}
                            outcome => return outcome,
                        }
                    }
                }
            }
            NodeKind::Doctype(_) => *doctype_seen = true,
            NodeKind::Element(el) if el.tag_name.eq_ignore_ascii_case("html") => {
                return if *doctype_seen {
                    Scan::Done
                } else {
                    Scan::Html(el)
                };
            }
            _ => {}
        }
    }
    Scan::Continue
}

/// Returns `true` if the block opens with `{% block %}`, marking the file as a partial.
/// Other root-level blocks (`{% if %}`, `{% for %}`, ...) are legitimate in full documents.
fn is_block_partial(block: &JinjaBlock<'_, Node<'_>>) -> bool {
    matches!(
        block.body.first(),
        Some(JinjaTagOrChildren::Tag(tag)) if parse_jinja_tag_name(tag) == "block"
    )
}

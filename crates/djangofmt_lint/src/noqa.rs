//! Inline diagnostic suppression via `{# noqa: code #}` comments.
//!
//! A `{# noqa: rule #}` comment silences the listed rules on the node that
//! immediately follows it (whitespace aside). Anchoring the suppression to the
//! following node, rather than to a line or column, keeps it attached to the
//! offending markup across reformatting.

use std::ops::Range;

use markup_fmt::ast::{JinjaBlock, JinjaTagOrChildren, Node, NodeKind, Root};

use crate::LintDiagnostic;

/// Rule codes suppressed over a byte range of the source.
struct Suppression<'s> {
    range: Range<usize>,
    codes: Vec<&'s str>,
}

/// Drop diagnostics silenced by a `{# noqa: ... #}` comment on the preceding node.
pub fn filter_suppressed<'s>(
    source: &'s str,
    root: &Root<'s>,
    mut diagnostics: Vec<LintDiagnostic>,
) -> Vec<LintDiagnostic> {
    if diagnostics.is_empty() {
        return diagnostics;
    }

    let mut suppressions = Vec::new();
    collect(source, &root.children, &mut suppressions);
    if suppressions.is_empty() {
        return diagnostics;
    }

    diagnostics.retain(|diagnostic| {
        let offset = diagnostic.span.offset();
        let code = diagnostic.code.as_str();
        !suppressions.iter().any(|suppression| {
            suppression.range.contains(&offset) && suppression.codes.contains(&code)
        })
    });
    diagnostics
}

/// Walk sibling lists, recording each `noqa` comment against the node it precedes.
fn collect<'s>(source: &'s str, nodes: &[Node<'s>], out: &mut Vec<Suppression<'s>>) {
    for (index, node) in nodes.iter().enumerate() {
        match &node.kind {
            NodeKind::JinjaComment(comment) => record(source, comment.raw, nodes, index, out),
            NodeKind::Element(element) => collect(source, &element.children, out),
            NodeKind::JinjaBlock(block) => collect_block(source, block, out),
            _ => {}
        }
    }
}

fn collect_block<'s>(
    source: &'s str,
    block: &JinjaBlock<'s, Node<'s>>,
    out: &mut Vec<Suppression<'s>>,
) {
    for item in &block.body {
        if let JinjaTagOrChildren::Children(children) = item {
            collect(source, children, out);
        }
    }
}

/// Record a suppression for the meaningful node following the comment at `index`.
fn record<'s>(
    source: &'s str,
    raw: &'s str,
    nodes: &[Node<'s>],
    index: usize,
    out: &mut Vec<Suppression<'s>>,
) {
    let Some(codes) = parse_directive(raw) else {
        return;
    };
    // Only whitespace may sit between the comment and the node it guards.
    let Some(target) = nodes[index + 1..]
        .iter()
        .find(|node| !is_whitespace_text(node))
    else {
        return;
    };
    let start = offset_of(source, target.raw);
    out.push(Suppression {
        range: start..start + target.raw.len(),
        codes,
    });
}

fn is_whitespace_text(node: &Node<'_>) -> bool {
    matches!(node.kind, NodeKind::Text(_)) && node.raw.trim().is_empty()
}

/// Byte offset of `slice` within `source` (both must share the same allocation).
fn offset_of(source: &str, slice: &str) -> usize {
    slice.as_ptr() as usize - source.as_ptr() as usize
}

/// Parse a comment body into the list of suppressed rule codes.
///
/// Only an explicit `noqa: code[, code...]` form suppresses. A bare `noqa`
/// (no colon) or a `noqa:` with no codes returns `None`.
fn parse_directive(raw: &str) -> Option<Vec<&str>> {
    let codes = raw.trim().strip_prefix("noqa")?.strip_prefix(':')?;
    let codes: Vec<&str> = codes
        .split(',')
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .collect();
    (!codes.is_empty()).then_some(codes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Settings, check_ast};
    use markup_fmt::Language;
    use markup_fmt::parser::Parser;

    #[test]
    fn parse_explicit_codes() {
        assert_eq!(
            parse_directive(" noqa: invalid-attr-value "),
            Some(vec!["invalid-attr-value"])
        );
        assert_eq!(parse_directive("noqa: a, b ,c"), Some(vec!["a", "b", "c"]));
        assert_eq!(parse_directive("noqa:x"), Some(vec!["x"]));
    }

    #[test]
    fn reject_non_directives() {
        assert_eq!(parse_directive(" noqa "), None); // bare noqa: explicit codes only
        assert_eq!(parse_directive("noqa:"), None); // no codes
        assert_eq!(parse_directive("noqa: ,"), None); // only separators
        assert_eq!(parse_directive(" djangofmt:ignore "), None);
        assert_eq!(parse_directive("not noqa: x"), None); // must start with noqa
        assert_eq!(parse_directive("noqacode: x"), None); // colon must follow noqa
    }

    fn count_diagnostics(source: &str) -> usize {
        let mut parser = Parser::new(source, Language::Django, vec![]);
        let ast = parser.parse_root().expect("parse");
        check_ast(source, &ast, &Settings::all()).len()
    }

    #[test]
    fn noqa_suppresses_following_node() {
        // `<form method="yes">` trips `invalid-attr-value`; the comment silences it.
        assert_eq!(count_diagnostics("<form method=\"yes\"></form>"), 1);
        assert_eq!(
            count_diagnostics("{# noqa: invalid-attr-value #}\n<form method=\"yes\"></form>"),
            0
        );
        // A non-matching code leaves the diagnostic in place.
        assert_eq!(
            count_diagnostics("{# noqa: empty-attr-value #}\n<form method=\"yes\"></form>"),
            1
        );
    }
}

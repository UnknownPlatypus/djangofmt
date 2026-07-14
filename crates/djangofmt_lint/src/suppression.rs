//! Inline diagnostic suppression via `{# djangofmt: ignore[...] #}` comments.
//!
//! A `{# djangofmt: ignore[rule1, rule2] #}` comment silences the listed rules
//! on the node that immediately follows it (whitespace and other comments
//! aside, so directives can stack or carry an explanation). Anchoring the
//! suppression to the following node, rather than to a line or column, keeps it
//! attached to the offending markup across reformatting.
//!
//! A `{# djangofmt: file-ignore[rule1, rule2] #}` comment at the top of the
//! file silences the listed rules for the whole file.
//!
//! Rules must always be listed explicitly: there is no blanket form, and the
//! formatter's bare `djangofmt:ignore` directive is unrelated to lint
//! suppression.

use std::ops::Range;

use markup_fmt::ast::{JinjaBlock, JinjaTagOrChildren, Node, NodeKind, Root};

use crate::LintDiagnostic;

/// A parsed suppression directive.
#[derive(Debug, PartialEq)]
enum Directive<'s> {
    /// `djangofmt: ignore[...]` — suppress on the following node.
    Ignore(Vec<&'s str>),
    /// `djangofmt: file-ignore[...]` — suppress for the whole file.
    FileIgnore(Vec<&'s str>),
}

/// Rule codes suppressed over a byte range of the source.
struct Suppression<'s> {
    range: Range<usize>,
    codes: Vec<&'s str>,
}

/// Drop diagnostics silenced by `ignore[...]` / `file-ignore[...]` comments.
pub fn filter_suppressed<'s>(
    source: &'s str,
    root: &Root<'s>,
    mut diagnostics: Vec<LintDiagnostic>,
) -> Vec<LintDiagnostic> {
    // Cheap prescan: most files carry no directive at all, skip the AST walk.
    if diagnostics.is_empty() || !source.contains("djangofmt:") {
        return diagnostics;
    }

    let mut suppressions = Vec::new();
    if let Some(codes) = file_ignore_codes(&root.children) {
        suppressions.push(Suppression {
            range: 0..source.len(),
            codes,
        });
    }
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

/// Codes of a `file-ignore[...]` directive on the first non-whitespace node.
fn file_ignore_codes<'s>(nodes: &[Node<'s>]) -> Option<Vec<&'s str>> {
    let node = nodes.iter().find(|node| !is_whitespace_text(node))?;
    match parse_directive(comment_body(node)?) {
        Some(Directive::FileIgnore(codes)) => Some(codes),
        _ => None,
    }
}

/// Walk sibling lists, recording each `ignore[...]` comment against the node it precedes.
fn collect<'s>(source: &'s str, nodes: &[Node<'s>], out: &mut Vec<Suppression<'s>>) {
    for (index, node) in nodes.iter().enumerate() {
        if let Some(body) = comment_body(node) {
            record(source, body, nodes, index, out);
        } else {
            match &node.kind {
                NodeKind::Element(element) => collect(source, &element.children, out),
                NodeKind::JinjaBlock(block) => collect_block(source, block, out),
                _ => {}
            }
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
    let Some(Directive::Ignore(codes)) = parse_directive(raw) else {
        return;
    };
    // Whitespace and other comments may sit between the comment and the node
    // it guards: diagnostics never fire on comments, and skipping them lets
    // directives stack or carry an explanation comment.
    let Some(target) = nodes[index + 1..]
        .iter()
        .find(|node| !is_whitespace_text(node) && comment_body(node).is_none())
    else {
        return;
    };
    let start = offset_of(source, target.raw);
    out.push(Suppression {
        range: start..start + target.raw.len(),
        codes,
    });
}

const fn comment_body<'s>(node: &Node<'s>) -> Option<&'s str> {
    match &node.kind {
        NodeKind::JinjaComment(comment) => Some(comment.raw),
        NodeKind::Comment(comment) => Some(comment.raw),
        _ => None,
    }
}

fn is_whitespace_text(node: &Node<'_>) -> bool {
    matches!(node.kind, NodeKind::Text(_)) && node.raw.trim().is_empty()
}

/// Byte offset of `slice` within `source` (both must share the same allocation).
fn offset_of(source: &str, slice: &str) -> usize {
    slice.as_ptr() as usize - source.as_ptr() as usize
}

/// Parse a comment body into a suppression directive.
///
/// Grammar: `djangofmt:`, optional whitespace, then `ignore[...]` or
/// `file-ignore[...]` with a non-empty comma-separated rule list and nothing
/// after the closing bracket. Anything else — including the formatter's bare
/// `djangofmt:ignore` — is not a lint directive.
fn parse_directive(raw: &str) -> Option<Directive<'_>> {
    let rest = raw.trim().strip_prefix("djangofmt:")?.trim_start();
    let (file_level, rest) = match rest.strip_prefix("file-ignore[") {
        Some(rest) => (true, rest),
        None => (false, rest.strip_prefix("ignore[")?),
    };
    let codes: Vec<&str> = rest
        .strip_suffix(']')?
        .split(',')
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .collect();
    if codes.is_empty() {
        return None;
    }
    Some(if file_level {
        Directive::FileIgnore(codes)
    } else {
        Directive::Ignore(codes)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Settings, check_ast};
    use markup_fmt::Language;
    use markup_fmt::parser::Parser;

    #[test]
    fn parse_node_directives() {
        assert_eq!(
            parse_directive(" djangofmt: ignore[invalid-attr-value] "),
            Some(Directive::Ignore(vec!["invalid-attr-value"]))
        );
        assert_eq!(
            parse_directive("djangofmt:ignore[a, b ,c]"),
            Some(Directive::Ignore(vec!["a", "b", "c"]))
        );
    }

    #[test]
    fn parse_file_directives() {
        assert_eq!(
            parse_directive(" djangofmt: file-ignore[invalid-syntax] "),
            Some(Directive::FileIgnore(vec!["invalid-syntax"]))
        );
        assert_eq!(
            parse_directive("djangofmt:file-ignore[a,b]"),
            Some(Directive::FileIgnore(vec!["a", "b"]))
        );
    }

    #[test]
    fn reject_non_directives() {
        assert_eq!(parse_directive(" djangofmt:ignore "), None); // formatter directive
        assert_eq!(parse_directive("djangofmt: ignore[]"), None); // explicit rules only
        assert_eq!(parse_directive("djangofmt: ignore[ , ]"), None); // only separators
        assert_eq!(parse_directive("djangofmt: ignore[a] trailing"), None); // nothing after bracket
        assert_eq!(parse_directive("ignore[a]"), None); // must start with djangofmt:
        assert_eq!(parse_directive("noqa: a"), None);
    }

    fn count_diagnostics(source: &str) -> usize {
        let mut parser = Parser::new(source, Language::Django, vec![]);
        let ast = parser.parse_root().expect("parse");
        check_ast(source, &ast, &Settings::all()).len()
    }

    #[test]
    fn ignore_suppresses_following_node() {
        // `<form method="yes">` trips `invalid-attr-value`; the comment silences it.
        assert_eq!(count_diagnostics("<form method=\"yes\"></form>"), 1);
        assert_eq!(
            count_diagnostics(
                "{# djangofmt: ignore[invalid-attr-value] #}\n<form method=\"yes\"></form>"
            ),
            0
        );
        // A non-matching code leaves the diagnostic in place.
        assert_eq!(
            count_diagnostics(
                "{# djangofmt: ignore[empty-attr-value] #}\n<form method=\"yes\"></form>"
            ),
            1
        );
    }

    #[test]
    fn ignore_reaches_past_intervening_comments() {
        // An explanation comment between the directive and its target is skipped.
        assert_eq!(
            count_diagnostics(
                "{# djangofmt: ignore[invalid-attr-value] #}\n{# TODO: fix this legacy form #}\n<form method=\"yes\"></form>"
            ),
            0
        );
        // Stacked directives all reach the same target.
        assert_eq!(
            count_diagnostics(
                "{# djangofmt: ignore[invalid-attr-value] #}\n{# djangofmt: ignore[empty-attr-value] #}\n<form method=\"yes\" id=\"\"></form>"
            ),
            0
        );
    }

    #[test]
    fn file_ignore_suppresses_whole_file() {
        let source = "{# djangofmt: file-ignore[invalid-attr-value] #}\n\
                      <form method=\"yes\"></form>\n\
                      <div><form method=\"put\"></form></div>";
        assert_eq!(count_diagnostics(source), 0);
        // Only honored at the top of the file.
        assert_eq!(
            count_diagnostics(
                "<p>hi</p>\n{# djangofmt: file-ignore[invalid-attr-value] #}\n<form method=\"yes\"></form>"
            ),
            1
        );
    }
}

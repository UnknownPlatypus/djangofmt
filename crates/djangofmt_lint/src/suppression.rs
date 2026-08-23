//! Inline diagnostic suppression via `{# djangofmt: ignore[...] #}` comments.
//!
//! A directive binds to the node that follows it — whitespace and other comments
//! aside, so directives can stack or carry an explanation. Anchoring to a node
//! rather than a line keeps the suppression on the right markup across
//! reformatting. `{# djangofmt: file-ignore[...] #}` as the file's first comment
//! covers the whole file instead.
//!
//! Rules are always listed explicitly, and only `{# #}` comments carry
//! directives: HTML comments survive in rendered output.

use std::ops::Range;

use markup_fmt::ast::{JinjaTagOrChildren, Node, NodeKind, Root};

use crate::rules::style::redirected_ignore;
use crate::rules::suspicious::{invalid_ignore_comment, unknown_ignore_code};
use crate::{Checker, LintDiagnostic};

/// `file-ignore[...]` code suppressing parse errors.
pub const INVALID_SYNTAX: &str = "invalid-syntax";

/// `file-ignore[...]` code skipping the formatter.
pub const FORMAT: &str = "format";

/// A parsed suppression directive.
#[derive(Debug, PartialEq, Eq)]
pub enum Directive<'s> {
    /// `djangofmt: ignore[...]` — suppress on the following node.
    Ignore(Vec<&'s str>),
    /// `djangofmt: file-ignore[...]` — suppress for the whole file.
    FileIgnore(Vec<&'s str>),
}

/// Rule codes suppressed over a byte range of the source.
pub struct Suppression<'s> {
    range: Range<usize>,
    codes: Vec<&'s str>,
}

/// Collect what each `djangofmt:` comment suppresses, linting misuse on the way.
#[must_use]
pub fn collect<'s>(
    source: &'s str,
    root: &Root<'s>,
    checker: &Checker<'_>,
) -> Vec<Suppression<'s>> {
    let mut suppressions = Vec::new();
    // Cheap prescan: most files carry no directive at all, skip the AST walk.
    if source.contains("djangofmt:") {
        let head = root.children.iter().find(|node| !is_whitespace_text(node));
        walk(source, &root.children, head, checker, &mut suppressions);
    }
    suppressions
}

/// Drop the diagnostics silenced by `suppressions`.
#[must_use]
pub fn filter(
    suppressions: &[Suppression<'_>],
    mut diagnostics: Vec<LintDiagnostic>,
) -> Vec<LintDiagnostic> {
    if suppressions.is_empty() {
        return diagnostics;
    }
    diagnostics.retain(|diagnostic| {
        let offset = diagnostic.span.offset() as usize;
        !suppressions.iter().any(|suppression| {
            suppression.range.contains(&offset) && suppression.codes.contains(&diagnostic.code)
        })
    });
    diagnostics
}

fn walk<'s>(
    source: &'s str,
    nodes: &[Node<'s>],
    head: Option<&Node<'s>>,
    checker: &Checker<'_>,
    out: &mut Vec<Suppression<'s>>,
) {
    for (index, node) in nodes.iter().enumerate() {
        match &node.kind {
            NodeKind::JinjaComment(comment) if is_lint_directive(comment.raw) => {
                let is_head = head.is_some_and(|head| std::ptr::eq(head, node));
                let directive = parse_directive(comment.raw);
                invalid_ignore_comment::check(directive.as_ref(), node.raw, is_head, checker);
                match directive {
                    Some(Directive::Ignore(codes)) => {
                        unknown_ignore_code::check(&codes, checker);
                        if let Some(range) = following_node(checker, &nodes[index + 1..]) {
                            out.push(Suppression { range, codes });
                        }
                    }
                    Some(Directive::FileIgnore(codes)) => {
                        unknown_ignore_code::check(&codes, checker);
                        if is_head {
                            out.push(Suppression {
                                range: 0..source.len(),
                                codes,
                            });
                        }
                    }
                    None => {}
                }
            }
            NodeKind::Comment(comment) => redirected_ignore::check(comment.raw, node.raw, checker),
            NodeKind::Element(element) => walk(source, &element.children, head, checker, out),
            NodeKind::JinjaBlock(block) => {
                for item in &block.body {
                    if let JinjaTagOrChildren::Children(children) = item {
                        walk(source, children, head, checker, out);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Byte range of the first node a directive comment can guard.
fn following_node(checker: &Checker<'_>, rest: &[Node<'_>]) -> Option<Range<usize>> {
    let target = rest
        .iter()
        .find(|node| !is_whitespace_text(node) && !is_comment(node))?;
    let start = checker.source_offset(target.raw);
    Some(start..start + target.raw.len())
}

/// A comment body stripped of Jinja's whitespace-control markers (`{#- ... -#}`),
/// which are part of the delimiter rather than of the directive.
fn directive_body(raw: &str) -> &str {
    raw.trim()
        .trim_start_matches(['-', '+'])
        .trim_end_matches('-')
        .trim()
}

/// Whether a comment body claims to be a lint directive.
///
/// The bare `djangofmt:ignore`, optionally followed by an explanation, is the
/// formatter's own directive and never a lint suppression.
fn is_lint_directive(body: &str) -> bool {
    let Some(after) = directive_body(body).strip_prefix("djangofmt:") else {
        return false;
    };
    !after
        .strip_prefix("ignore")
        .is_some_and(|rest| rest.chars().next().is_none_or(char::is_whitespace))
}

/// Parse a comment body into a suppression directive.
///
/// Grammar: `djangofmt:`, optional whitespace, then `ignore[...]` or
/// `file-ignore[...]` with a non-empty comma-separated rule list and nothing
/// after the closing bracket.
fn parse_directive(raw: &str) -> Option<Directive<'_>> {
    let rest = directive_body(raw).strip_prefix("djangofmt:")?.trim_start();
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

/// File-wide opt-outs declared by the leading comment of a file.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FileIgnores {
    /// The formatter skips the whole file (`file-ignore[format]`).
    pub format: bool,
    /// Parse errors are suppressed and the file skipped (`file-ignore[invalid-syntax]`).
    pub invalid_syntax: bool,
}

/// Opt-outs from the file's leading comment, read straight from the raw source
/// so they can be honored even when the file fails to parse.
///
/// The bare legacy `djangofmt:ignore` (in either comment style) predates rule
/// codes and opted the file out of everything: it maps to both flags.
#[must_use]
pub fn file_ignores(source: &str) -> FileIgnores {
    // A UTF-8 BOM is not Rust whitespace, so strip it explicitly.
    let trimmed = source
        .strip_prefix('\u{feff}')
        .unwrap_or(source)
        .trim_start();
    let jinja_body = leading_comment(trimmed, "{#", "#}");
    if let Some(body) = jinja_body.or_else(|| leading_comment(trimmed, "<!--", "-->"))
        && directive_body(body) == "djangofmt:ignore"
    {
        return FileIgnores {
            format: true,
            invalid_syntax: true,
        };
    }
    match jinja_body.map(parse_directive) {
        Some(Some(Directive::FileIgnore(codes))) => FileIgnores {
            format: codes.contains(&FORMAT),
            invalid_syntax: codes.contains(&INVALID_SYNTAX),
        },
        _ => FileIgnores::default(),
    }
}

/// The body of a leading `open`..`close` comment, if the text starts with one.
fn leading_comment<'s>(text: &'s str, open: &str, close: &str) -> Option<&'s str> {
    let body = text.strip_prefix(open)?;
    Some(&body[..body.find(close)?])
}

const fn is_comment(node: &Node<'_>) -> bool {
    matches!(node.kind, NodeKind::JinjaComment(_) | NodeKind::Comment(_))
}

/// A BOM is not Rust whitespace, but it does not displace the file's first comment.
fn is_whitespace_text(node: &Node<'_>) -> bool {
    matches!(node.kind, NodeKind::Text(_))
        && node
            .raw
            .chars()
            .all(|char| char.is_whitespace() || char == '\u{feff}')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Settings, lint_source};
    use markup_fmt::Language;

    /// `<form method="yes">` trips `invalid-attr-value`, `id=""` trips `empty-attr-value`.
    fn codes(source: &str) -> Vec<&'static str> {
        lint_source(source, Language::Django, &[], &Settings::all(), None)
            .expect("parse")
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    #[test]
    fn parse_directives() {
        assert_eq!(
            parse_directive(" djangofmt: ignore[a, b ,c] "),
            Some(Directive::Ignore(vec!["a", "b", "c"]))
        );
        assert_eq!(
            parse_directive("djangofmt:file-ignore[invalid-syntax]"),
            Some(Directive::FileIgnore(vec!["invalid-syntax"]))
        );
    }

    #[test]
    fn reject_non_directives() {
        assert_eq!(parse_directive(" djangofmt:ignore "), None); // formatter directive
        assert_eq!(parse_directive("djangofmt: ignore[]"), None); // explicit rules only
        assert_eq!(parse_directive("djangofmt: ignore[ , ]"), None); // only separators
        assert_eq!(parse_directive("djangofmt: ignore[a] trailing"), None); // nothing after bracket
        assert_eq!(parse_directive("ignore[a]"), None); // must start with djangofmt:
    }

    #[test]
    fn ignore_binds_to_the_following_node() {
        assert_eq!(
            codes("<form method=\"yes\"></form>"),
            ["invalid-attr-value"]
        );
        assert!(
            codes("{# djangofmt: ignore[invalid-attr-value] #}\n<form method=\"yes\"></form>")
                .is_empty()
        );
        // A non-matching code, a directive placed after the node, and a later
        // sibling all keep their diagnostic.
        assert_eq!(
            codes("{# djangofmt: ignore[empty-attr-value] #}\n<form method=\"yes\"></form>"),
            ["invalid-attr-value"]
        );
        assert_eq!(
            codes("<form method=\"yes\"></form>\n{# djangofmt: ignore[invalid-attr-value] #}"),
            ["invalid-attr-value"]
        );
        assert_eq!(
            codes(
                "{# djangofmt: ignore[invalid-attr-value] #}\n<form method=\"yes\"></form>\n<form method=\"put\"></form>"
            ),
            ["invalid-attr-value"]
        );
    }

    #[test]
    fn ignore_reaches_past_intervening_comments() {
        for filler in ["{# TODO: fix this legacy form #}", "<!-- legacy form -->"] {
            let source = format!(
                "{{# djangofmt: ignore[invalid-attr-value] #}}\n{filler}\n<form method=\"yes\"></form>"
            );
            assert!(!codes(&source).contains(&"invalid-attr-value"), "{filler}");
        }
        // Stacked directives all reach the same target.
        assert!(
            codes(
                "{# djangofmt: ignore[invalid-attr-value] #}\n{# djangofmt: ignore[empty-attr-value] #}\n<form method=\"yes\" id=\"\"></form>"
            )
            .is_empty()
        );
    }

    #[test]
    fn ignore_applies_inside_nested_nodes() {
        const GUARDED: &str =
            "{# djangofmt: ignore[invalid-attr-value] #}\n<form method=\"yes\"></form>";
        assert!(codes(&format!("<div>\n{GUARDED}\n</div>")).is_empty());
        assert!(codes(&format!("{{% if x %}}\n{GUARDED}\n{{% endif %}}")).is_empty());
    }

    #[test]
    fn jinja_whitespace_control_markers_are_not_part_of_the_directive() {
        assert!(
            codes("{#- djangofmt: ignore[invalid-attr-value] -#}\n<form method=\"yes\"></form>")
                .is_empty()
        );
        assert_eq!(
            file_ignores("{#- djangofmt: file-ignore[format] -#}"),
            FileIgnores {
                format: true,
                invalid_syntax: false,
            }
        );
    }

    #[test]
    fn a_bom_does_not_displace_the_first_comment() {
        assert!(
            codes(
                "\u{feff}{# djangofmt: file-ignore[invalid-attr-value] #}\n<form method=\"yes\"></form>"
            )
            .is_empty()
        );
    }

    #[test]
    fn html_comments_never_suppress() {
        // The rule still fires, plus a `redirected-ignore` on the comment.
        assert!(
            codes("<!-- djangofmt: ignore[invalid-attr-value] -->\n<form method=\"yes\"></form>")
                .contains(&"invalid-attr-value")
        );
    }

    #[test]
    fn file_ignore_is_honored_at_the_top_only() {
        assert!(
            codes(
                "{# djangofmt: file-ignore[invalid-attr-value] #}\n\
                 <form method=\"yes\"></form>\n\
                 <div><form method=\"put\"></form></div>"
            )
            .is_empty()
        );
        // Elsewhere it suppresses nothing and earns an `invalid-ignore-comment`.
        assert_eq!(
            codes(
                "<p>hi</p>\n{# djangofmt: file-ignore[invalid-attr-value] #}\n<form method=\"yes\"></form>"
            ),
            ["invalid-attr-value", "invalid-ignore-comment"]
        );
    }

    #[test]
    fn detect_file_level_opt_outs() {
        let all = FileIgnores {
            format: true,
            invalid_syntax: true,
        };
        let syntax_only = FileIgnores {
            format: false,
            invalid_syntax: true,
        };
        let format_only = FileIgnores {
            format: true,
            invalid_syntax: false,
        };

        assert_eq!(
            file_ignores("{# djangofmt: file-ignore[invalid-syntax] #}\n<div id=>"),
            syntax_only
        );
        assert_eq!(
            file_ignores("{# djangofmt: file-ignore[format] #}\n<div></div>"),
            format_only
        );
        assert_eq!(
            file_ignores("{# djangofmt: file-ignore[format, invalid-syntax] #}"),
            all
        );
        // A UTF-8 BOM or leading whitespace before the directive is tolerated.
        assert_eq!(
            file_ignores("\u{feff}{# djangofmt: file-ignore[invalid-syntax] #}\n<div id=>"),
            syntax_only
        );
        assert_eq!(
            file_ignores("\n  {# djangofmt: file-ignore[foo, invalid-syntax] #}\n<div id=>"),
            syntax_only
        );

        // The bare legacy directive opts out of everything, in both styles.
        assert_eq!(file_ignores("{# djangofmt:ignore #}\n<div id=>"), all);
        assert_eq!(file_ignores("<!-- djangofmt:ignore -->\n<div id=>"), all);
        assert_eq!(file_ignores("{#  djangofmt:ignore  #}\n<div id=>"), all);

        // Bracketed directives only count in `{# #}` comments.
        assert_eq!(
            file_ignores("<!-- djangofmt: file-ignore[invalid-syntax] -->\n<div id=>"),
            FileIgnores::default()
        );
        // Lint codes, node-level directives and plain markup are not opt-outs.
        assert_eq!(
            file_ignores("{# djangofmt: file-ignore[missing-img-alt] #}\n<div id=>"),
            FileIgnores::default()
        );
        assert_eq!(
            file_ignores("{# djangofmt: ignore[invalid-syntax] #}\n<div id=>"),
            FileIgnores::default()
        );
        assert_eq!(file_ignores("<div id=>"), FileIgnores::default());
    }
}

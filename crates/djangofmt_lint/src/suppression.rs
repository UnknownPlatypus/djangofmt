//! Diagnostic suppression via `{# djangofmt: ignore[...] #}` comments.
//!
//! - A directive guards the node that follows it.
//! - `file-ignore[...]` as the file's first comment covers the whole file.
//! - Only `{# #}` comments carry directives: HTML comments reach the client.

use std::ops::Range;
use std::str::FromStr;

use markup_fmt::ast::{JinjaTagOrChildren, Node, NodeKind, Root};
use smallvec::SmallVec;

use crate::registry::Rule;
use crate::rule_set::RuleSet;
use crate::rules::suspicious::{invalid_ignore_comment, unknown_ignore_code};
use crate::{Checker, LintDiagnostic};

/// The code that opts a node or file out of the formatter.
pub const FORMAT_CODE: &str = "format";

/// The node-level directive, also the formatter's own opt-out.
pub const IGNORE_DIRECTIVE: &str = "djangofmt:ignore";

/// Its file-wide sibling.
const FILE_IGNORE_DIRECTIVE: &str = "djangofmt:file-ignore";

/// A code accepted in `file-ignore[...]`; unknown codes are ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum FileIgnoreCode {
    /// Suppress parse errors: both commands skip the file.
    InvalidSyntax,
    /// Skip the formatter.
    Format,
}

/// The codes of a `ignore[...]` / `file-ignore[...]` comment body.
fn directive_codes<'s>(raw: &'s str, directive: &str) -> Option<Vec<&'s str>> {
    let codes: Vec<&str> = markup_fmt::match_directive(raw, directive)?
        .codes()
        .collect();
    (!codes.is_empty()).then_some(codes)
}

/// A parsed lint directive.
#[derive(Debug, PartialEq, Eq)]
pub enum Directive<'s> {
    /// `djangofmt: ignore[...]`, guarding the following node.
    Ignore(Vec<&'s str>),
    /// `djangofmt: file-ignore[...]`, covering the whole file.
    FileIgnore(Vec<&'s str>),
}

impl<'s> Directive<'s> {
    fn codes(&self) -> &[&'s str] {
        match self {
            Self::Ignore(codes) | Self::FileIgnore(codes) => codes,
        }
    }
}

/// Parse a comment that claims to be a lint directive; `None` means it is malformed.
fn parse_directive(raw: &str) -> Option<Directive<'_>> {
    directive_codes(raw, FILE_IGNORE_DIRECTIVE)
        .map(Directive::FileIgnore)
        .or_else(|| directive_codes(raw, IGNORE_DIRECTIVE).map(Directive::Ignore))
}

/// Whether a comment body claims to be a lint directive: `djangofmt:` followed by anything but
/// the formatter's bare `ignore`, which carries no lint codes.
fn is_lint_directive(raw: &str) -> bool {
    let claims = raw
        .trim_start()
        .trim_start_matches(['-', '+'])
        .trim_start()
        .strip_prefix("djangofmt")
        .is_some_and(|rest| rest.trim_start().starts_with(':'));
    claims && !markup_fmt::starts_with_directive(raw, IGNORE_DIRECTIVE)
}

/// Whether `raw` is the body of the file's leading comment, the only place `file-ignore` counts.
fn is_leading_comment(source: &str, raw: &str) -> bool {
    leading_comment(strip_bom(source).trim_start(), "{#", "#}")
        .is_some_and(|body| body.as_ptr() == raw.as_ptr())
}

/// Rule codes suppressed over byte ranges of the source.
pub struct Suppression<'s> {
    ranges: SmallVec<[Range<usize>; 2]>,
    codes: Vec<&'s str>,
}

/// Collect what each node-level `ignore[...]` comment suppresses, linting misuse on the way.
#[must_use]
pub fn collect<'s>(root: &Root<'s>, checker: &Checker<'_>) -> Vec<Suppression<'s>> {
    root.jinja_comments
        .iter()
        .filter_map(|raw| {
            if !is_lint_directive(raw) {
                return None;
            }
            // An attribute-position comment is no node: it neither guards nor gets linted.
            let (comment, rest) = locate(&root.children, checker.source_offset(raw), checker)?;
            let directive = parse_directive(raw);
            let is_head = is_leading_comment(checker.context().source(), raw);
            invalid_ignore_comment::check(directive.as_ref(), comment.raw, is_head, checker);
            let directive = directive?;
            unknown_ignore_code::check(directive.codes(), checker);
            let Directive::Ignore(codes) = directive else {
                return None;
            };
            let ranges = guarded_ranges(checker, rest)?;
            Some(Suppression { ranges, codes })
        })
        .collect()
}

/// The `{# #}` comment node whose body sits at `offset`, and the siblings following it.
fn locate<'n, 's>(
    mut nodes: &'n [Node<'s>],
    offset: usize,
    checker: &Checker<'_>,
) -> Option<(&'n Node<'s>, &'n [Node<'s>])> {
    let start = |node: &Node<'_>| checker.source_offset(node.raw);
    let end = |node: &Node<'_>| checker.source_offset(node.raw) + node.raw.len();
    loop {
        // Siblings are ordered and disjoint: the container is the first one ending past `offset`.
        let index = nodes.partition_point(|node| end(node) <= offset);
        let node = nodes.get(index).filter(|node| start(node) <= offset)?;
        if matches!(node.kind, NodeKind::JinjaComment(_)) {
            return Some((node, &nodes[index + 1..]));
        }
        nodes = child_slices(node).find(|children| {
            children
                .first()
                .zip(children.last())
                .is_some_and(|(first, last)| start(first) <= offset && offset < end(last))
        })?;
    }
}

/// Byte ranges a directive comment guards: the first node after it, minus that node's children.
/// For an element that is its opening and closing tags; for a block, its `{% %}` tags.
fn guarded_ranges(checker: &Checker<'_>, rest: &[Node<'_>]) -> Option<SmallVec<[Range<usize>; 2]>> {
    let target = rest.iter().find(|node| {
        !is_whitespace_text(node)
            && !matches!(node.kind, NodeKind::JinjaComment(_) | NodeKind::Comment(_))
    })?;
    let start = checker.source_offset(target.raw);
    let end = start + target.raw.len();
    let mut ranges = SmallVec::new();
    let mut cursor = start;
    for child in child_slices(target).flatten() {
        let child_start = checker.source_offset(child.raw);
        if child_start > cursor {
            ranges.push(cursor..child_start);
        }
        cursor = child_start + child.raw.len();
    }
    if cursor < end {
        ranges.push(cursor..end);
    }
    Some(ranges)
}

/// The child slices of a node: an element's children, or each branch of a block.
fn child_slices<'n, 's>(node: &'n Node<'s>) -> impl Iterator<Item = &'n [Node<'s>]> {
    let (children, body) = match &node.kind {
        NodeKind::Element(element) => (Some(element.children.as_slice()), None),
        NodeKind::JinjaBlock(block) => (None, Some(block.body.as_slice())),
        _ => (None, None),
    };
    children
        .into_iter()
        .chain(body.into_iter().flatten().filter_map(|item| match item {
            JinjaTagOrChildren::Children(children) => Some(children.as_slice()),
            JinjaTagOrChildren::Tag(_) => None,
        }))
}

/// Whitespace between a directive and its target does not displace the target.
fn is_whitespace_text(node: &Node<'_>) -> bool {
    matches!(node.kind, NodeKind::Text(_)) && node.raw.trim().is_empty()
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
            suppression.codes.contains(&diagnostic.code)
                && suppression
                    .ranges
                    .iter()
                    .any(|range| range.contains(&offset))
        })
    });
    diagnostics
}

/// File-wide opt-outs declared by the leading comment of a file.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FileIgnores {
    /// The formatter skips the whole file (`file-ignore[format]`).
    pub format: bool,
    /// Parse errors are suppressed and the file skipped (`file-ignore[invalid-syntax]`).
    pub invalid_syntax: bool,
}

impl FileIgnores {
    /// Opt-outs from the file's leading comment, read straight from the raw source
    /// so they can be honored even when the file fails to parse.
    #[must_use]
    pub fn parse(source: &str) -> Self {
        let source = strip_bom(source);

        // The bare legacy directive doubles as a node-level formatter directive,
        // so it is only file-level when nothing (not even whitespace) precedes it.
        let legacy_body =
            leading_comment(source, "{#", "#}").or_else(|| leading_comment(source, "<!--", "-->"));
        if let Some(body) = legacy_body
            && markup_fmt::starts_with_directive(body, IGNORE_DIRECTIVE)
        {
            return Self {
                format: true,
                invalid_syntax: true,
            };
        }
        leading_file_ignore_codes(source)
            .unwrap_or_default()
            .iter()
            .filter_map(|code| FileIgnoreCode::from_str(code).ok())
            .fold(Self::default(), |mut ignores, code| {
                match code {
                    FileIgnoreCode::Format => ignores.format = true,
                    FileIgnoreCode::InvalidSyntax => ignores.invalid_syntax = true,
                }
                ignores
            })
    }
}

/// The codes of the file's leading `{# djangofmt: file-ignore[...] #}` comment,
/// BOM and leading whitespace tolerated.
fn leading_file_ignore_codes(source: &str) -> Option<Vec<&str>> {
    leading_comment(strip_bom(source).trim_start(), "{#", "#}")
        .and_then(|body| directive_codes(body, FILE_IGNORE_DIRECTIVE))
}

/// Rules the file's leading `file-ignore[...]` comment turns off, so they never run at all.
#[must_use]
pub fn file_ignored_rules(source: &str) -> RuleSet {
    let mut rules = RuleSet::empty();
    if let Some(codes) = leading_file_ignore_codes(source) {
        for rule in codes.iter().filter_map(|code| Rule::from_str(code).ok()) {
            rules.insert(rule);
        }
    }
    rules
}

/// A UTF-8 BOM is not Rust whitespace, so strip it explicitly.
fn strip_bom(source: &str) -> &str {
    source.strip_prefix('\u{feff}').unwrap_or(source)
}

/// The body of a leading `open`..`close` comment, if the text starts with one.
fn leading_comment<'s>(text: &'s str, open: &str, close: &str) -> Option<&'s str> {
    let body = text.strip_prefix(open)?;
    Some(&body[..body.find(close)?])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Settings, lint_source};
    use markup_fmt::Language;
    use rstest::rstest;

    const ALL: FileIgnores = FileIgnores {
        format: true,
        invalid_syntax: true,
    };
    const SYNTAX_ONLY: FileIgnores = FileIgnores {
        format: false,
        invalid_syntax: true,
    };
    const FORMAT_ONLY: FileIgnores = FileIgnores {
        format: true,
        invalid_syntax: false,
    };
    const NONE: FileIgnores = FileIgnores {
        format: false,
        invalid_syntax: false,
    };

    /// The CLI builds the formatter's `ignore[format]` entry from `FORMAT_CODE`.
    #[test]
    fn format_code_matches_the_enum() {
        assert_eq!(
            FileIgnoreCode::from_str(FORMAT_CODE),
            Ok(FileIgnoreCode::Format)
        );
    }

    #[rstest]
    #[case::node(" djangofmt: ignore[a, b ,c] ", IGNORE_DIRECTIVE, &["a", "b", "c"])]
    #[case::file("djangofmt:file-ignore[invalid-syntax]", FILE_IGNORE_DIRECTIVE, &["invalid-syntax"])]
    #[case::spaced_colon("djangofmt : file-ignore[invalid-syntax]", FILE_IGNORE_DIRECTIVE, &["invalid-syntax"])]
    #[case::reason("djangofmt: ignore[a]: free-text reason", IGNORE_DIRECTIVE, &["a"])]
    fn parse_directive_codes(
        #[case] comment: &str,
        #[case] directive: &str,
        #[case] expected: &[&str],
    ) {
        assert_eq!(
            directive_codes(comment, directive).as_deref(),
            Some(expected)
        );
    }

    #[rstest]
    fn reject_non_directives(
        #[values(
            " djangofmt:ignore ",       // formatter directive
            "djangofmt: ignore[]",      // explicit rules only
            "djangofmt: ignore[ , ]",   // only separators
            "ignore[a]",                // must start with djangofmt:
            "djangofmt: file-ignore[a]" // the file-level sibling is not `ignore`
        )]
        comment: &str,
    ) {
        assert_eq!(directive_codes(comment, IGNORE_DIRECTIVE), None);
    }

    #[rstest]
    #[case::invalid_syntax("{# djangofmt: file-ignore[invalid-syntax] #}\n<div id=>", SYNTAX_ONLY)]
    #[case::format("{# djangofmt: file-ignore[format] #}\n<div></div>", FORMAT_ONLY)]
    #[case::both_codes("{# djangofmt: file-ignore[format, invalid-syntax] #}", ALL)]
    // Jinja whitespace-control markers are part of the delimiter.
    #[case::whitespace_control("{#- djangofmt: file-ignore[format] -#}", FORMAT_ONLY)]
    // Anything after the closing bracket is a free-text reason.
    #[case::reason("{# djangofmt: file-ignore[format]: vendored file #}", FORMAT_ONLY)]
    // A UTF-8 BOM or leading whitespace before the directive is tolerated.
    #[case::bom(
        "\u{feff}{# djangofmt: file-ignore[invalid-syntax] #}\n<div id=>",
        SYNTAX_ONLY
    )]
    #[case::leading_whitespace(
        "\n  {# djangofmt: file-ignore[foo, invalid-syntax] #}\n<div id=>",
        SYNTAX_ONLY
    )]
    // The bare legacy directive opts out of everything, in both styles,
    // with whitespace tolerated around the colon.
    #[case::legacy_jinja("{# djangofmt:ignore #}\n<div id=>", ALL)]
    #[case::legacy_html("<!-- djangofmt:ignore -->\n<div id=>", ALL)]
    #[case::legacy_spaced_colon("{# djangofmt : ignore #}\n<div id=>", ALL)]
    #[case::legacy_bom("\u{feff}{# djangofmt:ignore #}\n<div id=>", ALL)]
    // Preceded by whitespace, the bare directive is node-level, not file-level.
    #[case::legacy_after_newline("\n  {# djangofmt:ignore #}\n<div id=>", NONE)]
    #[case::legacy_after_space(" <!-- djangofmt:ignore -->\n<div id=>", NONE)]
    // Bracketed directives only count in `{# #}` comments.
    #[case::html_comment("<!-- djangofmt: file-ignore[invalid-syntax] -->\n<div id=>", NONE)]
    // Lint codes, node-level directives and plain markup are not opt-outs.
    #[case::lint_code("{# djangofmt: file-ignore[missing-img-alt] #}\n<div id=>", NONE)]
    #[case::node_level("{# djangofmt: ignore[invalid-syntax] #}\n<div id=>", NONE)]
    #[case::plain_markup("<div id=>", NONE)]
    fn detect_file_level_opt_outs(#[case] source: &str, #[case] expected: FileIgnores) {
        assert_eq!(FileIgnores::parse(source), expected);
    }

    /// Codes that name no rule, `format` and `invalid-syntax` included, narrow nothing.
    #[test]
    fn file_ignore_narrows_the_rule_set() {
        assert_eq!(
            file_ignored_rules("{# djangofmt: file-ignore[invalid-attr-value, format] #}"),
            RuleSet::from_rule(Rule::InvalidAttrValue)
        );
        // Below the file's top it narrows nothing.
        assert_eq!(
            file_ignored_rules("<p>hi</p>\n{# djangofmt: file-ignore[invalid-attr-value] #}"),
            RuleSet::empty()
        );
    }

    /// End to end: a narrowed rule never runs, anywhere in the file.
    #[test]
    fn file_ignored_rules_never_run() {
        assert!(
            codes(
                "{# djangofmt: file-ignore[invalid-attr-value] #}\n\
                 <form method=\"yes\"></form>\n\
                 <div><form method=\"put\"></form></div>"
            )
            .is_empty()
        );
    }

    /// `<form method="yes">` trips `invalid-attr-value`, `id=""` trips `empty-attr-value`.
    fn codes(source: &str) -> Vec<&'static str> {
        diagnostics(source)
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    /// Diagnostics by message, to tell apart two that share a code.
    fn messages(source: &str) -> Vec<String> {
        diagnostics(source)
            .into_iter()
            .map(|diagnostic| diagnostic.message.into_owned())
            .collect()
    }

    fn diagnostics(source: &str) -> Vec<LintDiagnostic> {
        lint_source(source, Language::Django, &[], &Settings::all(), None).expect("parse")
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
            messages(
                "{# djangofmt: ignore[invalid-attr-value] #}\n<form method=\"yes\"></form>\n<form method=\"put\"></form>"
            ),
            ["Invalid value 'put' for attribute 'method'."]
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
    fn ignore_guards_the_node_but_not_its_children() {
        // The directive guards the `<div>` tags, not the `<span>` nested inside them.
        assert_eq!(
            messages(
                "{# djangofmt: ignore[empty-attr-value] #}\n<div id=\"\"><span class=\"\">x</span></div>"
            ),
            ["Empty `class` attribute can be removed."]
        );
        // Likewise a block guards its own tags but not its body.
        assert!(
            codes(
                "{# djangofmt: ignore[untrimmed-blocktranslate] #}\n{% blocktranslate %}x{% endblocktranslate %}"
            )
            .is_empty()
        );
        assert_eq!(
            codes(
                "{# djangofmt: ignore[invalid-attr-value] #}\n{% if x %}<form method=\"yes\"></form>{% endif %}"
            ),
            ["invalid-attr-value"]
        );
    }

    #[test]
    fn whitespace_control_markers_do_not_break_suppression() {
        assert!(
            codes("{#- djangofmt: ignore[invalid-attr-value] -#}\n<form method=\"yes\"></form>")
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

    /// An attribute-position comment is indexed by the parser but is no node, so it has no target.
    #[test]
    fn attribute_position_comments_never_suppress() {
        assert_eq!(
            codes("<form {# djangofmt: ignore[invalid-attr-value] #} method=\"yes\"></form>"),
            ["invalid-attr-value"]
        );
    }
}

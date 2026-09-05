//! Diagnostic suppression via `{# djangofmt: ignore[...] #}` comments.
//!
//! - A directive guards the node that follows it.
//! - `file-ignore[...]` as the file's first comment covers the whole file.
//! - Only `{# #}` comments carry directives: HTML comments reach the client.

use std::ops::Range;
use std::str::FromStr;

pub use markup_fmt::ParseErrorKind;
use markup_fmt::ast::{JinjaTagOrChildren, Node, NodeKind, Root};
use smallvec::SmallVec;

use crate::Checker;
use crate::registry::Rule;
use crate::rule_set::RuleSet;

/// The code that opts a node or file out of the formatter.
pub const FORMAT_CODE: &str = "format";

/// The code that suppresses parse errors; only `file-ignore[...]` may carry it.
pub const INVALID_SYNTAX_CODE: &str = "invalid-syntax";

const NAMESPACE: &str = "djangofmt";
const IGNORE: &str = "ignore";
const FILE_IGNORE: &str = "file-ignore";

/// The formatter's directive, `NAMESPACE:IGNORE` spelled out for `markup_fmt` to match.
pub const IGNORE_DIRECTIVE: &str = "djangofmt:ignore";

/// What a `{# djangofmt: ... #}` comment asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum Directive<'s> {
    /// `ignore[...]`, guarding the following node.
    Ignore(Vec<&'s str>),
    /// `file-ignore[...]`, covering the whole file.
    FileIgnore(Vec<&'s str>),
    /// Addressed to djangofmt, yet neither of the above nor the formatter's bare `ignore`.
    Malformed(ParseErrorKind),
}

impl<'s> Directive<'s> {
    /// The codes listed, none for a malformed directive.
    pub fn codes(&self) -> &[&'s str] {
        match self {
            Self::Ignore(codes) | Self::FileIgnore(codes) => codes,
            Self::Malformed(_) => &[],
        }
    }
}

/// Parse a `{# #}` comment body as a lint directive, with the grammar the formatter uses.
///
/// `None` is a comment the linter has no say on: one not addressed to djangofmt, or the
/// formatter's bare `ignore`, which carries no lint codes.
fn parse(body: &str) -> Option<Directive<'_>> {
    let directive = match markup_fmt::parse_directive(body, NAMESPACE, &[IGNORE, FILE_IGNORE])? {
        Ok(directive) => directive,
        Err(error) => return Some(Directive::Malformed(error)),
    };
    Some(match (directive.keyword, directive.codes) {
        (IGNORE, codes) if codes.is_empty() => return None,
        (IGNORE, codes) => Directive::Ignore(codes),
        (_, codes) if codes.is_empty() => Directive::Malformed(ParseErrorKind::MissingCodes),
        (_, codes) => Directive::FileIgnore(codes),
    })
}

/// A `{# djangofmt: ... #}` comment as the linter reads it.
pub struct DirectiveComment<'s> {
    /// The whole comment, delimiters included: what a diagnostic points at and a fix deletes.
    pub raw: &'s str,
    /// What the comment asks for.
    pub directive: Directive<'s>,
    /// Whether it is the file's leading comment, the only place `file-ignore` counts.
    pub is_leading: bool,
    /// Byte ranges an `ignore[...]` guards; empty when nothing follows it.
    ranges: SmallVec<[Range<usize>; 2]>,
}

impl DirectiveComment<'_> {
    /// Whether the directive silences `code` reported at `offset`.
    fn suppresses(&self, code: &str, offset: usize) -> bool {
        self.ranges.iter().any(|range| range.contains(&offset))
            && self.directive.codes().contains(&code)
    }
}

/// Every lint directive of the file, in source order.
#[must_use]
pub fn collect<'s>(root: &Root<'s>, checker: &Checker<'s>) -> Vec<DirectiveComment<'s>> {
    let source = checker.context().source();
    root.jinja_comments
        .iter()
        .filter_map(|body| {
            let directive = parse(body)?;
            let offset = checker.source_offset(body);
            // The body sits between `{#` and `#}`; an unterminated comment runs to the end.
            let start = offset - "{#".len();
            let end = (offset + body.len() + "#}".len()).min(source.len());
            let ranges = match directive {
                // At attribute position the comment is no node, so it guards nothing.
                Directive::Ignore(_) => siblings_after(&root.children, offset, checker)
                    .map_or_else(SmallVec::new, |rest| guarded_ranges(checker, rest)),
                Directive::FileIgnore(_) | Directive::Malformed(_) => SmallVec::new(),
            };
            Some(DirectiveComment {
                raw: &source[start..end],
                directive,
                is_leading: source[..start]
                    .trim_start_matches('\u{feff}')
                    .trim_start()
                    .is_empty(),
                ranges,
            })
        })
        .collect()
}

/// Drop from `checker` the diagnostics the directives silence.
pub fn apply(directives: &[DirectiveComment<'_>], checker: &Checker<'_>) {
    if directives.is_empty() {
        return;
    }
    checker.context().retain_diagnostics(|diagnostic| {
        let offset = diagnostic.span.offset() as usize;
        !directives
            .iter()
            .any(|comment| comment.suppresses(diagnostic.code, offset))
    });
}

/// The siblings following the `{# #}` comment node whose body sits at `offset`.
fn siblings_after<'n, 's>(
    mut nodes: &'n [Node<'s>],
    offset: usize,
    checker: &Checker<'_>,
) -> Option<&'n [Node<'s>]> {
    let start = |node: &Node<'_>| checker.source_offset(node.raw);
    let end = |node: &Node<'_>| checker.source_end(node.raw);
    loop {
        // Siblings are ordered and disjoint: the container is the first one ending past `offset`.
        let index = nodes.partition_point(|node| end(node) <= offset);
        let node = nodes.get(index).filter(|node| start(node) <= offset)?;
        if matches!(node.kind, NodeKind::JinjaComment(_)) {
            return Some(&nodes[index + 1..]);
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
fn guarded_ranges(checker: &Checker<'_>, rest: &[Node<'_>]) -> SmallVec<[Range<usize>; 2]> {
    let mut ranges = SmallVec::new();
    let Some(target) = rest.iter().find(|node| {
        !is_whitespace_text(node)
            && !matches!(node.kind, NodeKind::JinjaComment(_) | NodeKind::Comment(_))
    }) else {
        return ranges;
    };
    let start = checker.source_offset(target.raw);
    let end = checker.source_end(target.raw);
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
    ranges
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
            && markup_fmt::matches_directive(body, IGNORE_DIRECTIVE)
        {
            return Self {
                format: true,
                invalid_syntax: true,
            };
        }
        leading_file_ignore_codes(source)
            .unwrap_or_default()
            .iter()
            .fold(Self::default(), |mut ignores, code| {
                match *code {
                    FORMAT_CODE => ignores.format = true,
                    INVALID_SYNTAX_CODE => ignores.invalid_syntax = true,
                    _ => {}
                }
                ignores
            })
    }
}

/// The codes of the file's leading `{# djangofmt: file-ignore[...] #}` comment.
fn leading_file_ignore_codes(source: &str) -> Option<Vec<&str>> {
    match parse(leading_jinja_comment(source)?)? {
        Directive::FileIgnore(codes) => Some(codes),
        Directive::Ignore(_) | Directive::Malformed(_) => None,
    }
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

/// The body of the file's leading `{# #}` comment, BOM and whitespace before it tolerated.
fn leading_jinja_comment(source: &str) -> Option<&str> {
    leading_comment(strip_bom(source).trim_start(), "{#", "#}")
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
    use crate::{LintDiagnostic, Settings, lint_source};
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

    /// The formatter matches the spelled-out directive; the linter parses namespace and keyword.
    #[test]
    fn ignore_directive_spells_the_namespace_and_keyword() {
        assert_eq!(IGNORE_DIRECTIVE, format!("{NAMESPACE}:{IGNORE}"));
    }

    #[rstest]
    #[case::node(" djangofmt: ignore[a, b ,c] ", Directive::Ignore(vec!["a", "b", "c"]))]
    #[case::file("djangofmt:file-ignore[invalid-syntax]", Directive::FileIgnore(vec!["invalid-syntax"]))]
    #[case::spaced_colon("djangofmt : file-ignore[a]", Directive::FileIgnore(vec!["a"]))]
    #[case::spaced_list("djangofmt: file-ignore [a]", Directive::FileIgnore(vec!["a"]))]
    #[case::spaced_node_list("djangofmt: ignore [a]", Directive::Ignore(vec!["a"]))]
    #[case::trailing_comma("djangofmt: ignore[a,]", Directive::Ignore(vec!["a"]))]
    #[case::reason("djangofmt: ignore[a]: free-text reason", Directive::Ignore(vec!["a"]))]
    #[case::whitespace_control("- djangofmt: ignore[a] -", Directive::Ignore(vec!["a"]))]
    #[case::unknown_keyword(
        "djangofmt: silence[a]",
        Directive::Malformed(ParseErrorKind::UnknownKeyword)
    )]
    #[case::bare_file_ignore(
        "djangofmt: file-ignore",
        Directive::Malformed(ParseErrorKind::MissingCodes)
    )]
    #[case::empty_list(
        "djangofmt: ignore[]",
        Directive::Malformed(ParseErrorKind::MissingCodes)
    )]
    #[case::unclosed_list(
        "djangofmt: ignore[a",
        Directive::Malformed(ParseErrorKind::MissingBracket)
    )]
    #[case::missing_comma(
        "djangofmt: ignore[a b]",
        Directive::Malformed(ParseErrorKind::MissingComma)
    )]
    #[case::only_separators(
        "djangofmt: ignore[ , ]",
        Directive::Malformed(ParseErrorKind::InvalidCode)
    )]
    #[case::numeric_code(
        "djangofmt: ignore[1x]",
        Directive::Malformed(ParseErrorKind::InvalidCode)
    )]
    fn parse_directives(#[case] comment: &'static str, #[case] expected: Directive<'static>) {
        assert_eq!(parse(comment), Some(expected));
    }

    #[rstest]
    fn skip_comments_the_linter_has_no_say_on(
        #[values(
            " djangofmt:ignore ",                // the formatter's directive
            "djangofmt:ignore this is generated", // with a reason
            "djangofmt ignore[a]",               // missing colon
            "ignore[a]",                         // not addressed to djangofmt
            "See djangofmt: https://example.com" // merely mentioning it
        )]
        comment: &str,
    ) {
        assert_eq!(parse(comment), None);
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
            ["Invalid value `put` for attribute `method`"]
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
            ["Empty `class` attribute"]
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

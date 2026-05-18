//! Line-level `{# noqa #}` suppression for djangofmt lint diagnostics.
//!
//! This module mirrors the architecture of ruff's noqa subsystem
//! (`astral-sh/ruff` `crates/ruff_linter/src/noqa.rs` +
//! `crates/ruff_linter/src/checkers/noqa.rs`):
//!
//! 1. Rules emit diagnostics unconditionally during the AST walk.
//! 2. After the walk, [`check_noqa`] inspects every `{# … #}` comment in the
//!    AST, builds a sorted list of per-line [`NoqaDirectiveLine`]s, and
//!    removes any diagnostic whose line carries a matching directive.
//!
//! Grammar (v1):
//! - `{# noqa #}` — blanket suppression for the line.
//! - `{# noqa: invalid-attr-value #}` — suppress one rule.
//! - `{# noqa: invalid-attr-value, untrimmed-blocktranslate #}` — multiple
//!   rules. Comma- or whitespace-separated.
//! - `noqa` is case-insensitive; codes are case-sensitive (must match
//!   `Rule`'s kebab-case display name).
//! - Leading/trailing comment text is tolerated: `{# explanation noqa: X #}`
//!   suppresses `X` on its line.
//!
//! Future extensions (kept under the `noqa` namespace, see issue #289):
//! - File-level (`{# noqa: file #}`).
//! - Block ranges (`{# noqa: disable=rule #}` / `{# noqa: enable=rule #}`).
//! - `unused-noqa` rule (the `matches: Vec<Rule>` field is already populated
//!   during suppression to power it).

use std::str::FromStr;

use markup_fmt::ast::{Element, JinjaBlock, JinjaTagOrChildren, Node, NodeKind, Root};
use miette::SourceSpan;

use crate::LintDiagnostic;
use crate::registry::Rule;

/// Parsed noqa directive.
///
/// Ruff equivalent: `Directive` in `ruff/crates/ruff_linter/src/noqa.rs`.
///
/// Differences from ruff:
/// - No `Directive::None`; absence is represented by `Option<Directive>` at
///   the call site.
/// - Codes are kebab-case rule names instead of `[A-Z]+[0-9]+` short codes.
#[derive(Debug)]
enum Directive<'a> {
    /// `{# noqa #}` (no codes) — suppress every diagnostic on the line.
    All,
    /// `{# noqa: rule, rule #}` — suppress only the listed rules.
    Codes(Vec<Code<'a>>),
}

/// A single rule code parsed from a `noqa:` list, with its byte range in the
/// source for diagnostic-quality error reporting.
///
/// Ruff equivalent: `Code` in `ruff/crates/ruff_linter/src/noqa.rs`.
#[derive(Debug)]
struct Code<'a> {
    text: &'a str,
    /// Byte range of this code inside the source. Reserved for the future
    /// `unused-noqa` rule, which needs to point at individual unused codes.
    #[expect(
        dead_code,
        reason = "reserved for unused-noqa diagnostics (issue #289 follow-up)"
    )]
    range: SourceSpan,
}

impl<'a> Code<'a> {
    const fn text(&self) -> &'a str {
        self.text
    }
}

/// One `{# noqa #}` directive attached to a physical source line.
///
/// Ruff equivalent: `NoqaDirectiveLine` in
/// `ruff/crates/ruff_linter/src/noqa.rs`.
///
/// `matches` tracks which rules this directive actually silenced; it is
/// populated during [`check_noqa`] and reserved for the future
/// `unused-noqa` rule (issue #289 follow-up).
#[derive(Debug)]
struct NoqaDirectiveLine<'a> {
    /// Physical-line range that the directive governs.
    line_range: SourceSpan,
    directive: Directive<'a>,
    /// Rules this directive actually silenced. Populated during
    /// [`check_noqa`]; reserved for the future `unused-noqa` rule.
    matches: Vec<Rule>,
}

/// Sorted collection of per-line noqa directives for a file.
///
/// Ruff equivalent: `NoqaDirectives` in
/// `ruff/crates/ruff_linter/src/noqa.rs`.
///
/// Built by walking `JinjaComment` nodes in the AST. Lookup is by byte
/// offset → enclosing-line range via [`LineIndex`], then binary search on
/// the sorted `lines` vector.
#[derive(Debug, Default)]
struct NoqaDirectives<'a> {
    lines: Vec<NoqaDirectiveLine<'a>>,
}

impl<'a> NoqaDirectives<'a> {
    fn from_ast(source: &'a str, ast: &Root<'a>, line_index: &LineIndex) -> Self {
        let mut directives = Self::default();
        for node in &ast.children {
            directives.visit_node(source, node, line_index);
        }
        // Traversal order is source order, but be defensive in case future
        // passes reorder nodes (binary search relies on sortedness).
        directives
            .lines
            .sort_by_key(|line| line.line_range.offset());
        directives
    }

    fn visit_node(&mut self, source: &'a str, node: &Node<'a>, line_index: &LineIndex) {
        match &node.kind {
            NodeKind::JinjaComment(_) => {
                if let Some(directive) = parse_jinja_comment(source, node) {
                    let line_range = line_index.line_range_at(source_offset(source, node.raw));
                    self.lines.push(NoqaDirectiveLine {
                        line_range,
                        directive,
                        matches: Vec::new(),
                    });
                }
            }
            NodeKind::Element(element) => self.visit_element(source, element, line_index),
            NodeKind::JinjaBlock(block) => self.visit_jinja_block(source, block, line_index),
            _ => {}
        }
    }

    fn visit_element(&mut self, source: &'a str, element: &Element<'a>, line_index: &LineIndex) {
        for child in &element.children {
            self.visit_node(source, child, line_index);
        }
    }

    fn visit_jinja_block(
        &mut self,
        source: &'a str,
        block: &JinjaBlock<'a, Node<'a>>,
        line_index: &LineIndex,
    ) {
        for item in &block.body {
            if let JinjaTagOrChildren::Children(children) = item {
                for child in children {
                    self.visit_node(source, child, line_index);
                }
            }
        }
    }

    /// Find the directive (if any) whose line contains `offset`. Returns a
    /// mutable reference so `check_noqa` can record matches on the line for
    /// the future `unused-noqa` rule.
    ///
    /// Ruff equivalent: `NoqaDirectives::find_line_with_directive_mut`.
    fn find_line_with_directive_mut(
        &mut self,
        offset: usize,
    ) -> Option<&mut NoqaDirectiveLine<'a>> {
        let idx = self
            .lines
            .binary_search_by(|line| {
                let start = line.line_range.offset();
                let end = start + line.line_range.len();
                if offset < start {
                    std::cmp::Ordering::Greater
                } else if offset >= end {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .ok()?;
        Some(&mut self.lines[idx])
    }
}

/// Newline-offset index over the source, used to project a byte offset
/// onto its enclosing physical line.
///
/// Ruff equivalent: `Locator` / `NoqaMapping` combined responsibility in
/// `ruff/crates/ruff_linter/src/noqa.rs` and `directives.rs`. djangofmt
/// has no multi-line string-literal projection to worry about, so this is
/// just a thin newline index — no `NoqaMapping` needed.
struct LineIndex {
    /// Byte offsets of every `\n` in the source.
    newlines: Vec<usize>,
    source_len: usize,
}

impl LineIndex {
    fn new(source: &str) -> Self {
        let newlines = source
            .bytes()
            .enumerate()
            .filter_map(|(i, b)| (b == b'\n').then_some(i))
            .collect();
        Self {
            newlines,
            source_len: source.len(),
        }
    }

    /// The half-open `[start, end)` byte range of the line containing `offset`.
    /// `end` excludes the trailing `\n`.
    fn line_range_at(&self, offset: usize) -> SourceSpan {
        let next_nl = self.newlines.partition_point(|&nl| nl < offset);
        let end = self
            .newlines
            .get(next_nl)
            .copied()
            .unwrap_or(self.source_len);
        let start = if next_nl == 0 {
            0
        } else {
            self.newlines[next_nl - 1] + 1
        };
        (start, end - start).into()
    }
}

/// Parse a `JinjaComment` node into a `Directive`, or return `None` if the
/// comment carries no `noqa` keyword (or is malformed in a way that we log
/// and discard).
fn parse_jinja_comment<'a>(source: &'a str, node: &Node<'a>) -> Option<Directive<'a>> {
    // `Node.raw` includes the surrounding `{#` / `#}` delimiters.
    // `JinjaComment.raw` is the inner body but the wrapper kind is opaque
    // here, so we strip delimiters off `Node.raw` ourselves.
    let body = node.raw.strip_prefix("{#")?.strip_suffix("#}")?;
    let body_offset = source_offset(source, body);
    lex_inline_noqa(body, body_offset)
}

/// Byte offset of `slice` within `source`. Pointer arithmetic identical to
/// `LintContext::source_offset` (kept local to avoid the public
/// `Checker`/`LintContext` plumbing).
fn source_offset(source: &str, slice: &str) -> usize {
    let src_start = source.as_ptr() as usize;
    let slice_start = slice.as_ptr() as usize;
    debug_assert!(
        slice_start >= src_start && slice_start + slice.len() <= src_start + source.len(),
        "slice must be a subslice of source"
    );
    slice_start - src_start
}

/// Hand-written noqa lexer.
///
/// Ruff equivalent: `lex_inline_noqa` in
/// `ruff/crates/ruff_linter/src/noqa.rs`.
///
/// Differences from ruff:
/// - Operates on a comment *body* (delimiters already stripped) rather than
///   a token stream, since djangofmt parses comments into AST nodes.
/// - Code grammar is `[a-zA-Z][a-zA-Z0-9_-]*` (kebab-case rule names)
///   rather than `[A-Z]+[0-9]+`.
fn lex_inline_noqa(body: &str, body_offset: usize) -> Option<Directive<'_>> {
    NoqaCursor { body, body_offset }.find_noqa()
}

struct NoqaCursor<'a> {
    body: &'a str,
    body_offset: usize,
}

impl<'a> NoqaCursor<'a> {
    /// Scan the body for a case-insensitive `noqa` keyword on a word
    /// boundary and parse what follows. Returns `None` if no keyword is
    /// found or if the payload is malformed.
    fn find_noqa(self) -> Option<Directive<'a>> {
        let mut start = 0;
        while start < self.body.len() {
            let idx = find_keyword_ci(&self.body[start..], "noqa")?;
            let kw_start = start + idx;
            let kw_end = kw_start + "noqa".len();
            if is_word_boundary(self.body, kw_start, kw_end) {
                return self.parse_after_keyword(kw_end);
            }
            start = kw_end;
        }
        None
    }

    fn parse_after_keyword(&self, mut pos: usize) -> Option<Directive<'a>> {
        pos += count_leading_inline_ws(&self.body[pos..]);

        let bytes = self.body.as_bytes();
        if pos >= bytes.len() || bytes[pos] != b':' {
            // No colon → blanket directive.
            return Some(Directive::All);
        }

        // Consume `:` and following whitespace.
        pos += 1;
        pos += count_leading_inline_ws(&self.body[pos..]);

        let codes = self.parse_code_list(pos);
        if codes.is_empty() {
            tracing::warn!(
                "ignoring malformed noqa directive: `noqa:` was not followed by any rule codes"
            );
            return None;
        }
        Some(Directive::Codes(codes))
    }

    fn parse_code_list(&self, start: usize) -> Vec<Code<'a>> {
        let mut codes = Vec::new();
        let bytes = self.body.as_bytes();
        let mut pos = start;
        loop {
            // Skip separators (commas + whitespace).
            while pos < bytes.len() && (bytes[pos] == b',' || bytes[pos].is_ascii_whitespace()) {
                pos += 1;
            }
            let code_start = pos;
            while pos < bytes.len() && is_code_char(bytes[pos]) {
                pos += 1;
            }
            if pos == code_start {
                break;
            }
            let code_text = &self.body[code_start..pos];
            codes.push(Code {
                text: code_text,
                range: (self.body_offset + code_start, code_text.len()).into(),
            });
        }
        codes
    }
}

#[inline]
fn count_leading_inline_ws(s: &str) -> usize {
    s.bytes().take_while(|&b| b == b' ' || b == b'\t').count()
}

#[inline]
const fn is_code_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

#[inline]
fn is_word_boundary(s: &str, kw_start: usize, kw_end: usize) -> bool {
    let bytes = s.as_bytes();
    let left_ok = kw_start == 0 || !is_word_char(bytes[kw_start - 1]);
    let right_ok = kw_end == bytes.len() || !is_word_char(bytes[kw_end]);
    left_ok && right_ok
}

#[inline]
const fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Case-insensitive ASCII substring search.
fn find_keyword_ci(haystack: &str, needle: &str) -> Option<usize> {
    debug_assert!(needle.is_ascii());
    let nbytes = needle.as_bytes();
    let hbytes = haystack.as_bytes();
    if hbytes.len() < nbytes.len() {
        return None;
    }
    'outer: for start in 0..=(hbytes.len() - nbytes.len()) {
        for i in 0..nbytes.len() {
            if !hbytes[start + i].eq_ignore_ascii_case(&nbytes[i]) {
                continue 'outer;
            }
        }
        return Some(start);
    }
    None
}

/// Post-pass: drop every diagnostic suppressed by a `{# noqa #}` directive.
///
/// Ruff equivalent: `check_noqa` in
/// `ruff/crates/ruff_linter/src/checkers/noqa.rs`.
///
/// Mutates `diagnostics` in place, preserving the relative order of the
/// diagnostics that remain (via `Vec::retain`).
pub fn check_noqa(diagnostics: &mut Vec<LintDiagnostic>, source: &str, ast: &Root<'_>) {
    if diagnostics.is_empty() {
        return;
    }
    let line_index = LineIndex::new(source);
    let mut directives = NoqaDirectives::from_ast(source, ast, &line_index);
    if directives.lines.is_empty() {
        return;
    }

    diagnostics.retain(|diagnostic| {
        let offset = diagnostic.span.offset();
        let Some(line) = directives.find_line_with_directive_mut(offset) else {
            return true;
        };
        match &line.directive {
            Directive::All => {
                if let Ok(rule) = Rule::from_str(&diagnostic.code) {
                    line.matches.push(rule);
                }
                false
            }
            Directive::Codes(codes) => {
                let suppressed = codes.iter().any(|c| c.text() == diagnostic.code);
                if suppressed && let Ok(rule) = Rule::from_str(&diagnostic.code) {
                    line.matches.push(rule);
                }
                !suppressed
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn lex(body: &str) -> Option<Directive<'_>> {
        lex_inline_noqa(body, 0)
    }

    #[rstest]
    #[case::blanket(" noqa ")]
    #[case::blanket_no_space("noqa")]
    #[case::blanket_uppercase(" NOQA ")]
    #[case::blanket_mixed_case(" NoQa ")]
    fn blanket_directive_parses(#[case] body: &str) {
        match lex(body) {
            Some(Directive::All) => {}
            other => panic!("expected blanket directive for {body:?}, got {other:?}"),
        }
    }

    #[rstest]
    #[case::single(" noqa: invalid-attr-value ", &["invalid-attr-value"][..])]
    #[case::no_space_after_colon(" noqa:invalid-attr-value ", &["invalid-attr-value"][..])]
    #[case::two_codes_comma(
        " noqa: invalid-attr-value, untrimmed-blocktranslate ",
        &["invalid-attr-value", "untrimmed-blocktranslate"][..],
    )]
    #[case::two_codes_whitespace(
        " noqa: invalid-attr-value untrimmed-blocktranslate ",
        &["invalid-attr-value", "untrimmed-blocktranslate"][..],
    )]
    #[case::leading_text(" some explanation noqa: invalid-attr-value ", &["invalid-attr-value"][..])]
    fn codes_directive_parses(#[case] body: &str, #[case] expect_codes: &[&str]) {
        match lex(body) {
            Some(Directive::Codes(codes)) => {
                let got: Vec<&str> = codes.iter().map(Code::text).collect();
                assert_eq!(got, expect_codes);
            }
            other => panic!("expected codes directive for {body:?}, got {other:?}"),
        }
    }

    #[rstest]
    #[case::no_keyword(" just a comment ")]
    #[case::substring_only(" prenoqasuffix ")]
    #[case::colon_no_codes(" noqa: ")]
    #[case::colon_only_separators(" noqa: , , ")]
    fn directive_rejected(#[case] body: &str) {
        assert!(
            lex(body).is_none(),
            "did not expect a directive for {body:?}"
        );
    }

    #[test]
    fn line_index_finds_enclosing_range() {
        // Offsets: 'a'=0, '\n'=1, 'b'=2,3,4 '\n'=5, 'c'=6,7 '\n'=8
        let src = "a\nbbb\ncc\n";
        let idx = LineIndex::new(src);

        let r = idx.line_range_at(3);
        assert_eq!(r.offset(), 2);
        assert_eq!(r.len(), 3); // "bbb"

        let r = idx.line_range_at(0);
        assert_eq!(r.offset(), 0);
        assert_eq!(r.len(), 1); // "a"

        // Past last newline returns the source-end range with len=0.
        let r = idx.line_range_at(9);
        assert_eq!(r.offset(), 9);
        assert_eq!(r.len(), 0);
    }
}

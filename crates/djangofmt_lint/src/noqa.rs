//! Node-attached `{# noqa #}` suppression for djangofmt lint diagnostics.
//!
//! # Why not line-based?
//!
//! djangofmt is a formatter first. The formatter relocates comments onto their
//! own line — comments are not "text-like" nodes, so the printer inserts a hard
//! line break before them (`format_children_*` in `markup_fmt`'s `printer.rs`). A
//! trailing same-line `{# noqa #}` is therefore moved off the diagnostic's line
//! during formatting and silently stops suppressing anything:
//!
//! ```text
//! <form method="put"></form> {# noqa #}   ->   <form method="put"></form>
//!                                              {# noqa #}   <- moved away
//! ```
//!
//! # Node attachment
//!
//! Suppression is instead bound to the **following node**, exactly like the
//! formatter's own `djangofmt:ignore` directive (`should_ignore_node` in
//! `markup_fmt`'s `printer.rs`): a directive comment on its own line immediately
//! before a node governs that node's entire subtree. This placement is stable
//! across formatting (verified: it survives attribute-wrapping and
//! re-indentation), and matches the model used by `prettier-ignore` and Biome's
//! formatter suppressions.
//!
//! ```text
//! {# noqa: invalid-attr-value #}
//! <form method="put">...</form>
//! ```
//!
//! Because every `Node::raw` spans the full subtree (open tag → children →
//! close tag), a directive before an element covers diagnostics on its
//! attributes and descendants too.
//!
//! # Grammar
//!
//! The directive *grammar* mirrors ruff's noqa lexer (`astral-sh/ruff`
//! `crates/ruff_linter/src/noqa.rs`):
//! - `{# noqa #}` — blanket suppression for the governed node.
//! - `{# noqa: invalid-attr-value #}` — suppress one rule.
//! - `{# noqa: invalid-attr-value, untrimmed-blocktranslate #}` — multiple
//!   rules, comma- or whitespace-separated.
//! - `noqa` is case-insensitive; codes are case-sensitive (must match a
//!   `Rule`'s kebab-case display name).
//! - Leading comment text is tolerated: `{# explanation, noqa: X #}`.
//!
//! # Reserved follow-ups (issue #289)
//!
//! Block ranges (`{# noqa: off #}` / `{# noqa: on #}`), file-level
//! (`{# noqa: file #}`), the `unused-noqa` rule, and `<!-- noqa -->` HTML
//! comments are deliberately out of scope for this version.

use markup_fmt::ast::{Element, JinjaBlock, JinjaTagOrChildren, Node, NodeKind, Root};

use crate::LintDiagnostic;

/// A parsed noqa directive.
///
/// Ruff equivalent: `Directive` in `ruff/crates/ruff_linter/src/noqa.rs`.
///
/// Differences from ruff:
/// - No `Directive::None`; absence is `Option<Directive>` at the call site.
/// - Codes are kebab-case rule names instead of `[A-Z]+[0-9]+` short codes.
#[derive(Debug)]
enum Directive<'a> {
    /// `{# noqa #}` (no codes) — suppress every diagnostic on the governed node.
    All,
    /// `{# noqa: rule, rule #}` — suppress only the listed rules.
    Codes(Vec<&'a str>),
}

impl Directive<'_> {
    /// Whether this directive silences the rule named `code`.
    fn suppresses(&self, code: &str) -> bool {
        match self {
            Directive::All => true,
            Directive::Codes(codes) => codes.contains(&code),
        }
    }
}

/// A directive together with the byte range of the node it governs.
struct GovernedDirective<'a> {
    /// Full byte range of the governed node's subtree (`Node::raw`).
    range: std::ops::Range<usize>,
    directive: Directive<'a>,
}

/// All `{# noqa #}` directives in a file, each bound to the node it governs.
///
/// Ruff equivalent: `NoqaDirectives` in
/// `ruff/crates/ruff_linter/src/noqa.rs` — but keyed by AST node range rather
/// than physical line, since djangofmt's formatter moves comments between lines.
#[derive(Default)]
struct NoqaDirectives<'a> {
    directives: Vec<GovernedDirective<'a>>,
}

impl<'a> NoqaDirectives<'a> {
    fn from_ast(source: &'a str, ast: &Root<'a>) -> Self {
        let mut this = Self::default();
        this.visit_children(source, &ast.children);
        this
    }

    fn visit_children(&mut self, source: &'a str, children: &[Node<'a>]) {
        for (index, node) in children.iter().enumerate() {
            match &node.kind {
                NodeKind::JinjaComment(_) => {
                    if let Some(directive) = parse_directive(node.raw)
                        && let Some(target) = governed_node(children, index)
                    {
                        let start = source_offset(source, target.raw);
                        self.directives.push(GovernedDirective {
                            range: start..start + target.raw.len(),
                            directive,
                        });
                    }
                }
                NodeKind::Element(element) => self.visit_element(source, element),
                NodeKind::JinjaBlock(block) => self.visit_jinja_block(source, block),
                _ => {}
            }
        }
    }

    fn visit_element(&mut self, source: &'a str, element: &Element<'a>) {
        self.visit_children(source, &element.children);
    }

    fn visit_jinja_block(&mut self, source: &'a str, block: &JinjaBlock<'a, Node<'a>>) {
        for item in &block.body {
            if let JinjaTagOrChildren::Children(children) = item {
                self.visit_children(source, children);
            }
        }
    }

    /// Whether `offset` is covered by a directive that silences `code`.
    fn suppresses(&self, offset: usize, code: &str) -> bool {
        self.directives
            .iter()
            .any(|d| d.range.contains(&offset) && d.directive.suppresses(code))
    }
}

/// The node a directive at `index` governs: the next sibling, skipping a single
/// whitespace-only text node between the directive and its target.
///
/// Mirrors (inverted) `markup_fmt`'s `should_ignore_node` in `printer.rs`, which
/// pairs `djangofmt:ignore` with the element that immediately follows it and
/// tolerates one whitespace-only text node in between.
fn governed_node<'b, 'a>(children: &'b [Node<'a>], index: usize) -> Option<&'b Node<'a>> {
    match children.get(index + 1) {
        Some(Node {
            kind: NodeKind::Text(text),
            ..
        }) if is_all_ascii_whitespace(text.raw) => children.get(index + 2),
        other => other,
    }
}

fn is_all_ascii_whitespace(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_whitespace())
}

/// Parse a Jinja comment's raw text (`{# ... #}`) into a directive, or `None`
/// if it carries no `noqa` keyword (or is malformed and discarded).
fn parse_directive(raw: &str) -> Option<Directive<'_>> {
    // `Node::raw` includes the surrounding delimiters; strip them to get the body.
    let body = raw.trim().strip_prefix("{#")?.strip_suffix("#}")?;
    lex_inline_noqa(body)
}

/// Byte offset of `slice` within `source`. Pointer arithmetic identical to
/// `LintContext::source_offset` (kept local to avoid the public plumbing).
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
/// - Operates on a comment *body* (delimiters already stripped) rather than a
///   token stream, since djangofmt parses comments into AST nodes.
/// - Code grammar is `[a-zA-Z][a-zA-Z0-9_-]*` (kebab-case rule names) rather
///   than `[A-Z]+[0-9]+`.
fn lex_inline_noqa(body: &str) -> Option<Directive<'_>> {
    NoqaCursor { body }.find_noqa()
}

struct NoqaCursor<'a> {
    body: &'a str,
}

impl<'a> NoqaCursor<'a> {
    /// Scan the body for a case-insensitive `noqa` keyword on a word boundary
    /// and parse what follows. Returns `None` if no keyword is found or if the
    /// payload is malformed.
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

    fn parse_code_list(&self, start: usize) -> Vec<&'a str> {
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
            codes.push(&self.body[code_start..pos]);
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
/// diagnostics that remain (via `Vec::retain`). Wiring this into [`check_ast`]
/// gates `--fix` too, since the fixer derives its diagnostics from `check_ast`.
///
/// [`check_ast`]: crate::check_ast
pub fn check_noqa(diagnostics: &mut Vec<LintDiagnostic>, source: &str, ast: &Root<'_>) {
    if diagnostics.is_empty() {
        return;
    }
    let directives = NoqaDirectives::from_ast(source, ast);
    if directives.directives.is_empty() {
        return;
    }

    diagnostics
        .retain(|diagnostic| !directives.suppresses(diagnostic.span.offset(), &diagnostic.code));
}

#[cfg(test)]
mod tests {
    use super::*;
    use markup_fmt::Language;
    use markup_fmt::parser::Parser;
    use rstest::rstest;

    fn codes<'a>(directive: &Directive<'a>) -> Vec<&'a str> {
        match directive {
            Directive::Codes(codes) => codes.clone(),
            Directive::All => panic!("expected codes directive, got blanket"),
        }
    }

    #[rstest]
    #[case::blanket(" noqa ")]
    #[case::blanket_no_space("noqa")]
    #[case::blanket_uppercase(" NOQA ")]
    #[case::blanket_mixed_case(" NoQa ")]
    fn blanket_directive_parses(#[case] body: &str) {
        assert!(
            matches!(lex_inline_noqa(body), Some(Directive::All)),
            "expected blanket directive for {body:?}"
        );
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
    fn codes_directive_parses(#[case] body: &str, #[case] expect: &[&str]) {
        let directive = lex_inline_noqa(body).expect("expected a directive");
        assert_eq!(codes(&directive), expect);
    }

    #[rstest]
    #[case::no_keyword(" just a comment ")]
    #[case::substring_only(" prenoqasuffix ")]
    #[case::colon_no_codes(" noqa: ")]
    #[case::colon_only_separators(" noqa: , , ")]
    fn directive_rejected(#[case] body: &str) {
        assert!(
            lex_inline_noqa(body).is_none(),
            "did not expect a directive for {body:?}"
        );
    }

    /// Worked example of the whole mechanism: a `{# noqa #}` on its own line
    /// before an element governs that element's full subtree, while a trailing
    /// directive (the position the formatter would move a comment to) governs
    /// nothing before it.
    #[test]
    fn governs_following_node_not_preceding() {
        let source = concat!(
            "{# noqa: invalid-attr-value #}\n",
            "<form method=\"put\">x</form>\n",
            "<form method=\"put\">y</form>\n",
            "{# noqa: invalid-attr-value #}",
        );
        let mut parser = Parser::new(source, Language::Django, vec![]);
        let ast = parser.parse_root().unwrap();
        let directives = NoqaDirectives::from_ast(source, &ast);

        // One directive recorded: the leading one (the trailing one governs nothing).
        assert_eq!(directives.directives.len(), 1);

        // The invalid value of the FIRST form is suppressed...
        let first = source.find("put").unwrap();
        assert!(directives.suppresses(first, "invalid-attr-value"));
        // ...but an unrelated code on that same node is not.
        assert!(!directives.suppresses(first, "untrimmed-blocktranslate"));

        // The SECOND form (only preceded by the first form, then a trailing
        // directive after it) is not suppressed.
        let second = source[first + 3..].find("put").unwrap() + first + 3;
        assert!(!directives.suppresses(second, "invalid-attr-value"));
    }
}

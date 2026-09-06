//! Source-text helpers shared by suppression handling and the rules.

use crate::Checker;

/// Delimiters of a `{# #}` template comment.
pub const TEMPLATE_COMMENT_OPEN: &str = "{#";
pub const TEMPLATE_COMMENT_CLOSE: &str = "#}";

/// Delimiters of an `<!-- -->` HTML comment.
pub const HTML_COMMENT_OPEN: &str = "<!--";
pub const HTML_COMMENT_CLOSE: &str = "-->";

/// A UTF-8 BOM is not Rust whitespace, so strip it explicitly.
#[must_use]
pub fn strip_bom(source: &str) -> &str {
    source.strip_prefix('\u{feff}').unwrap_or(source)
}

/// The body of a leading `{# #}` comment, if `text` starts with one.
pub fn leading_template_comment(text: &str) -> Option<&str> {
    leading_comment(text, TEMPLATE_COMMENT_OPEN, TEMPLATE_COMMENT_CLOSE)
}

/// The body of a leading `<!-- -->` comment, if `text` starts with one.
pub fn leading_html_comment(text: &str) -> Option<&str> {
    leading_comment(text, HTML_COMMENT_OPEN, HTML_COMMENT_CLOSE)
}

fn leading_comment<'s>(text: &'s str, open: &str, close: &str) -> Option<&'s str> {
    let body = text.strip_prefix(open)?;
    Some(&body[..body.find(close)?])
}

/// The whole comment around `body`, delimiters included.
/// An unterminated comment runs to the end of the source.
pub fn enclosing_comment<'s>(
    checker: &Checker<'s>,
    body: &str,
    open: &str,
    close: &str,
) -> &'s str {
    let source = checker.context().source();
    let start = checker.source_offset(body) - open.len();
    let end = (checker.source_end(body) + close.len()).min(source.len());
    &source[start..end]
}

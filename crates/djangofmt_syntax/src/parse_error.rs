use markup_fmt::{SyntaxError, SyntaxErrorKind};
use miette::{Diagnostic, NamedSource, SourceCode, SourceSpan, SpanContents};
use thiserror::Error;

use crate::diagnostic::span;

/// Build a span that miette can always draw a caret under.
/// If the parse error is at EOF, place the caret just before
fn eof_aware_span(source: &str, pos: usize) -> SourceSpan {
    if pos < source.len() {
        span(pos, 0)
    } else {
        source.char_indices().next_back().map_or_else(
            || span(pos, 0),
            |(start, _)| span(start, source.len() - start),
        )
    }
}

/// Where a jinja tag name starts after `{%`, skipping whitespace-trim markers.
fn jinja_name_pos(source: &str, after_brace: usize) -> usize {
    let rest = &source[after_brace..];
    after_brace + (rest.len() - rest.trim_start_matches(['+', '-']).trim_start().len())
}

#[derive(Debug, Diagnostic, Error)]
#[error("{message}")]
pub struct ParseError {
    pub message: String,
    #[source_code]
    src: NamedSource<String>,
    #[label("here")]
    span: SourceSpan,
    #[help]
    hint: String,
}

/// Escape hatches suggested when a file cannot be parsed.
pub const SKIP_FILE_HINT: &str = "Add `{# djangofmt: file-ignore[invalid-syntax] #}` at the top of this file, or list it in `extend-exclude`, to skip it.";

/// Elements whose end tag HTML5 allows omitting, while djangofmt keeps requiring it.
/// <https://html.spec.whatwg.org/multipage/syntax.html#optional-tags>
const OPTIONAL_END_TAG_ELEMENTS: [&str; 19] = [
    "body", "caption", "colgroup", "dd", "dt", "head", "html", "li", "optgroup", "option", "p",
    "rp", "rt", "tbody", "td", "tfoot", "th", "thead", "tr",
];

/// A missing close tag has two very different causes, so point each one at its own limitation.
fn close_tag_hint(tag_name: &str) -> String {
    if OPTIONAL_END_TAG_ELEMENTS
        .iter()
        .any(|element| element.eq_ignore_ascii_case(tag_name))
    {
        format!(
            "HTML5 allows omitting `</{tag_name}>`, but djangofmt requires it. Add the explicit close tag.\n\
             https://unknownplatypus.github.io/djangofmt/docs/known-limitations/#omitted-end-tags"
        )
    } else {
        format!(
            "If a `</{tag_name}>` does exist, it must live in the same block as the opening tag.\n\
             https://unknownplatypus.github.io/djangofmt/docs/known-limitations/#conditional-openclose-tags"
        )
    }
}

impl ParseError {
    /// `name` labels the source in the rendered report.
    #[must_use]
    pub fn new(name: impl AsRef<str>, source: String, err: &SyntaxError) -> Self {
        let (message, hint, span) = match &err.kind {
            // Point to the opening tag instead of where the error was detected (which is always the end of the file)
            SyntaxErrorKind::ExpectCloseTag { tag_name, pos, .. } => (
                format!("expected close tag for opening tag <{tag_name}>"),
                Some(close_tag_hint(tag_name)),
                // `pos` is the `<`; the caret covers the tag name.
                span(pos + 1, tag_name.len()),
            ),
            SyntaxErrorKind::ExpectJinjaBlockEnd { tag_name, pos, .. } => (
                format!("unclosed {{% {tag_name} %}} block."),
                Some("Check for invalid HTML syntax inside the block that might prevent finding the end tag.".into()),
                // `pos` is just past the `{%`; the caret covers the tag name.
                span(jinja_name_pos(&source, *pos), tag_name.len()),
            ),
            SyntaxErrorKind::SelfClosingNonVoidElement(tag_name) => (
                err.kind.to_string(),
                Some(format!(
                    "Browsers ignore the `/` and read `<{tag_name}/>` as an opening `<{tag_name}>`. \
                     Write `<{tag_name}></{tag_name}>` for an empty element, or `</{tag_name}>` if this was meant to close one."
                )),
                // `pos` is the `<`; the caret covers the tag name.
                span(err.pos + 1, tag_name.len()),
            ),
            _ => (
                err.kind.to_string(),
                None,
                eof_aware_span(&source, err.pos),
            ),
        };
        Self {
            message,
            src: NamedSource::new(name, source),
            span,
            // Every parse error offers a way out, unless the kind had something better to say.
            hint: hint.unwrap_or_else(|| SKIP_FILE_HINT.to_string()),
        }
    }

    /// 1-based line and column the error points at.
    #[must_use]
    pub fn location(&self) -> (usize, usize) {
        self.src
            .read_span(&self.span, 0, 0)
            .map_or((0, 0), |contents| {
                (contents.line() + 1, contents.column() + 1)
            })
    }
}

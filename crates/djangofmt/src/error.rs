use miette::{Diagnostic, NamedSource, SourceCode, SourceSpan, SpanContents};
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::fs::relativize_path;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error, Diagnostic)]
pub enum Error {
    // -- Externals
    #[error(transparent)]
    #[diagnostic(code(djangofmt::io_error))]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    #[diagnostic(code(djangofmt::resolve_error))]
    Resolve(String),
}

#[must_use]
pub fn path_display(path: Option<&PathBuf>) -> String {
    path.map_or_else(|| "<unknown>".to_string(), relativize_path)
}

/// Build a span that miette can always draw a caret under.
/// If the parse error is at EOF, place the caret just before
fn eof_aware_span(source: &str, pos: usize) -> SourceSpan {
    if pos < source.len() {
        djangofmt_lint::span(pos, 0)
    } else {
        source.char_indices().next_back().map_or_else(
            || djangofmt_lint::span(pos, 0),
            |(start, _)| djangofmt_lint::span(start, source.len() - start),
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
    pub path: Option<PathBuf>,
    pub message: String,
    #[source_code]
    src: NamedSource<String>,
    #[label("here")]
    span: SourceSpan,
    #[help]
    hint: Option<String>,
}

/// An error that can occur while processing a file in a command (format or check).
#[derive(Debug, Error, Diagnostic)]
pub enum CommandError {
    #[error("Failed to read {path}: {err}", path = path_display(.0.as_ref()), err = .1)]
    Read(Option<PathBuf>, #[source] io::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Parse(ParseError),
    #[error("Failed to write {path}: {err}", path = path_display(.0.as_ref()), err = .1)]
    Write(Option<PathBuf>, #[source] io::Error),
}

impl CommandError {
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Parse(err) => err.path.as_deref(),
            Self::Read(path, _) | Self::Write(path, _) => path.as_deref(),
        }
    }

    /// Render as a single `path:line:column: message` line.
    #[must_use]
    pub fn concise(&self) -> String {
        match self {
            Self::Parse(err) => {
                let (line, column) = err.location();
                let path = path_display(err.path.as_ref());
                format!("{path}:{line}:{column}: {}", err.message)
            }
            Self::Read(..) | Self::Write(..) => self.to_string(),
        }
    }
}

/// Escape hatches suggested when a file cannot be parsed.
pub const SKIP_FILE_HINT: &str = "Add `{# djangofmt: file-ignore[invalid-syntax] #}` at the top of this file, or list it in `extend-exclude`, to skip it.";

impl ParseError {
    #[must_use]
    pub fn new(path: Option<PathBuf>, source: String, err: &markup_fmt::FormatError) -> Self {
        let (message, hint, span) = match err {
            markup_fmt::FormatError::Syntax(syntax_err) => {
                match &syntax_err.kind {
                    // Point to the opening tag instead of where the error was detected (which is always the end of the file)
                    markup_fmt::SyntaxErrorKind::ExpectCloseTag { tag_name, pos, .. } => (
                        format!("expected close tag for opening tag <{tag_name}>"),
                        Some(format!(
                            "If a `</{tag_name}>` does exist, it must live in the same block as the opening tag: \
                             https://unknownplatypus.github.io/djangofmt/docs/known-limitations/#conditional-openclose-tags"
                        )),
                        // `pos` is the `<`; the caret covers the tag name.
                        djangofmt_lint::span(pos + 1, tag_name.len()),
                    ),
                    markup_fmt::SyntaxErrorKind::ExpectJinjaBlockEnd { tag_name, pos, .. } => (
                        format!("unclosed {{% {tag_name} %}} block."),
                        Some("Check for invalid HTML syntax inside the block that might prevent finding the end tag.".into()),
                        // `pos` is just past the `{%`; the caret covers the tag name.
                        djangofmt_lint::span(jinja_name_pos(&source, *pos), tag_name.len()),
                    ),
                    _ => (
                        syntax_err.kind.to_string(),
                        None,
                        eof_aware_span(&source, syntax_err.pos),
                    ),
                }
            }
            markup_fmt::FormatError::External(errors) => {
                let msg = errors
                    .iter()
                    .map(|e| format!("{e:?}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                (format!("external formatter error: {msg}"), None, 0.into())
            }
        };
        let name = path_display(path.as_ref());
        Self {
            path,
            message,
            src: NamedSource::new(name, source),
            span,
            hint,
        }
    }

    /// Append a help line to the error, preserving any existing hint.
    #[must_use]
    pub fn with_hint(mut self, hint: &str) -> Self {
        self.hint = Some(self.hint.take().map_or_else(
            || hint.to_string(),
            |existing| format!("{existing}\n{hint}"),
        ));
        self
    }

    /// 1-based line and column the error points at.
    fn location(&self) -> (usize, usize) {
        self.src
            .read_span(&self.span, 0, 0)
            .map_or((0, 0), |contents| {
                (contents.line() + 1, contents.column() + 1)
            })
    }
}

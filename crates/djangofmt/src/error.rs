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

/// Byte offset where 1-based `line` starts.
fn line_start(source: &str, line: usize) -> usize {
    if line <= 1 {
        0
    } else {
        source
            .match_indices('\n')
            .nth(line - 2)
            .map_or(0, |(nl, _)| nl + 1)
    }
}

/// Invert `markup_fmt`'s `pos_to_line_col`: columns are byte-based, biased +1 on
/// line 1 and +2 on later lines, and reported as 0 when pos is on the last line.
fn line_col_to_pos(source: &str, line: usize, column: usize) -> Option<usize> {
    match (line, column) {
        (_, 0) => None,
        (1, column) => Some(column - 1),
        (line, column) => Some((line_start(source, line) + column).saturating_sub(2)),
    }
}

/// True if the char at `pos` could extend a tag name.
fn is_name_char_at(source: &str, pos: usize) -> bool {
    source[pos..]
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
}

/// Where a jinja tag name starts after `{%`, skipping whitespace-trim markers.
fn jinja_name_pos(source: &str, after_brace: usize) -> usize {
    let rest = &source[after_brace..];
    after_brace + (rest.len() - rest.trim_start_matches(['+', '-']).trim_start().len())
}

fn name_span(source: &str, name_pos: Option<usize>, len: usize) -> SourceSpan {
    name_pos.map_or_else(
        || eof_aware_span(source, source.len()),
        |pos| djangofmt_lint::span(pos, len),
    )
}

/// Caret span on the name of the unclosed `<tag_name ...>` reported at (line, column).
fn open_tag_span(source: &str, tag_name: &str, line: usize, column: usize) -> SourceSpan {
    // markup_fmt points at the `<`; move onto the name.
    let name_pos = line_col_to_pos(source, line, column)
        .map(|pos| pos + 1)
        .or_else(|| {
            // The column was lost (last line): find the innermost `<tag_name` on that line.
            let start = line_start(source, line);
            source[start..]
                .rmatch_indices(&format!("<{tag_name}"))
                .map(|(idx, _)| start + idx + 1)
                .find(|&pos| !is_name_char_at(source, pos + tag_name.len()))
        });
    name_span(source, name_pos, tag_name.len())
}

/// Caret span on the name of the unclosed `{% tag_name %}` reported at (line, column).
fn jinja_tag_span(source: &str, tag_name: &str, line: usize, column: usize) -> SourceSpan {
    // markup_fmt points just past the `{%`; move onto the name.
    let name_pos = line_col_to_pos(source, line, column)
        .map(|pos| jinja_name_pos(source, pos))
        .or_else(|| {
            // The column was lost (last line): find the innermost `{% tag_name` on that line.
            let start = line_start(source, line);
            source[start..]
                .rmatch_indices("{%")
                .map(|(idx, _)| jinja_name_pos(source, start + idx + 2))
                .find(|&pos| {
                    source[pos..].starts_with(tag_name)
                        && !is_name_char_at(source, pos + tag_name.len())
                })
        });
    name_span(source, name_pos, tag_name.len())
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

impl ParseError {
    #[must_use]
    pub fn new(path: Option<PathBuf>, source: String, err: &markup_fmt::FormatError) -> Self {
        let (message, hint, span) = match err {
            markup_fmt::FormatError::Syntax(syntax_err) => {
                match &syntax_err.kind {
                    // Point to the opening tag instead of where the error was detected (which is always the end of the file)
                    markup_fmt::SyntaxErrorKind::ExpectCloseTag {
                        tag_name,
                        line,
                        column,
                    } => (
                        format!("expected close tag for opening tag <{tag_name}>"),
                        Some(format!(
                            "If a `</{tag_name}>` does exist, it must live in the same block as the opening tag: \
                             https://unknownplatypus.github.io/djangofmt/docs/known-limitations/#conditional-openclose-tags"
                        )),
                        open_tag_span(&source, tag_name, *line, *column),
                    ),
                    markup_fmt::SyntaxErrorKind::ExpectJinjaBlockEnd {
                        tag_name,
                        line,
                        column,
                    } => (
                        format!("unclosed {{% {tag_name} %}} block."),
                        Some("Check for invalid HTML syntax inside the block that might prevent finding the end tag.".into()),
                        jinja_tag_span(&source, tag_name, *line, *column),
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

    /// 1-based line and column the error points at.
    fn location(&self) -> (usize, usize) {
        self.src
            .read_span(&self.span, 0, 0)
            .map_or((0, 0), |contents| {
                (contents.line() + 1, contents.column() + 1)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::line_col_to_pos;

    #[test]
    fn inverts_markup_fmt_line_col_encoding() {
        // Byte layout: a=0, é=1..3, b=3, \n=4, c=5, d=6, \n=7, e=8.
        let source = "aéb\ncd\ne";
        assert_eq!(line_col_to_pos(source, 1, 1), Some(0));
        // Line-1 columns are byte-based and biased by 1.
        assert_eq!(line_col_to_pos(source, 1, 4), Some(3));
        // A position exactly on a newline reports that line's numbers.
        assert_eq!(line_col_to_pos(source, 1, 5), Some(4));
        // Columns on later lines are biased by 2.
        assert_eq!(line_col_to_pos(source, 2, 2), Some(5));
        assert_eq!(line_col_to_pos(source, 2, 4), Some(7));
        // The last line loses the column entirely.
        assert_eq!(line_col_to_pos(source, 3, 0), None);
    }
}

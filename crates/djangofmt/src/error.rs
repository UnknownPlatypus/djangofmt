use miette::{Diagnostic, NamedSource, SourceOffset, SourceSpan};
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error, Diagnostic)]
pub enum Error {
    // -- Externals
    #[error(transparent)]
    #[diagnostic(code(djangofmt::io_error))]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic(code(djangofmt::miette_error))]
    Miette(#[from] miette::InstallError),

    #[error("{0}")]
    #[diagnostic(code(djangofmt::resolve_error))]
    Resolve(String),
}

#[must_use]
pub fn path_display(path: Option<&PathBuf>) -> String {
    path.map_or_else(|| "<unknown>".to_string(), |p| p.display().to_string())
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
}

impl ParseError {
    #[must_use]
    pub fn new<E: std::fmt::Debug>(
        path: Option<PathBuf>,
        source: String,
        err: &markup_fmt::FormatError<E>,
    ) -> Self {
        let (message, hint, span) = match err {
            markup_fmt::FormatError::Syntax(syntax_err) => {
                match &syntax_err.kind {
                    // Point to the opening tag instead of where the error was detected (which is always the end of the file)
                    markup_fmt::SyntaxErrorKind::ExpectCloseTag {
                        tag_name,
                        line,
                        column,
                    } => (
                        format!("expected close tag for opening tag <{tag_name}>",),
                        None,
                        SourceSpan::new(SourceOffset::from_location(&source, *line, *column), tag_name.len()),
                    ),
                    markup_fmt::SyntaxErrorKind::ExpectJinjaBlockEnd {
                        tag_name,
                        line,
                        column,
                    } => (
                        format!("unclosed {{% {tag_name} %}} block."),
                        Some("Check for invalid HTML syntax inside the block that might prevent finding the end tag.".into()),
                        SourceSpan::new(SourceOffset::from_location(&source, *line, *column +1), tag_name.len()),
                    ),
                    _ => (syntax_err.kind.to_string(), None, syntax_err.pos.into()),
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
}

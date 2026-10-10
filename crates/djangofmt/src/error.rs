use djangofmt_formatter::panic::PanicError;
use djangofmt_syntax::ParseError;
use miette::Diagnostic;
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

/// A missing path means the source came from stdin, report it as the stdin sentinel `-`.
#[must_use]
pub fn path_display(path: Option<&Path>) -> String {
    path.map_or_else(|| crate::STDIN_SENTINEL.to_string(), relativize_path)
}

/// An error that can occur while processing a file in a command (format or check).
#[derive(Debug, Error, Diagnostic)]
pub enum CommandError {
    #[error("Failed to read {path}: {err}", path = path_display(.0.as_deref()), err = .1)]
    Read(Option<PathBuf>, #[source] io::Error),
    #[error("{1}")]
    #[diagnostic(forward(1))]
    Parse(Option<PathBuf>, ParseError),
    #[error("Failed to write {path}: {err}", path = path_display(.0.as_deref()), err = .1)]
    Write(Option<PathBuf>, #[source] io::Error),
    #[error(
        "Panicked while processing {path}: This indicates a bug in djangofmt. \
         If you could open an issue at {repo}/issues/new?title=%5BPanic%5D \
         with the file contents and the trace below, we'd be very appreciative!\n{err}",
        path = path_display(.0.as_deref()),
        repo = env!("CARGO_PKG_REPOSITORY"),
        err = .1
    )]
    Panic(Option<PathBuf>, Box<PanicError>),
}

impl CommandError {
    /// Render as a single `path:line:column: message` line.
    #[must_use]
    pub fn concise(&self) -> String {
        match self {
            Self::Parse(path, err) => {
                let (line, column) = err.location();
                let path = path_display(path.as_deref());
                format!("{path}:{line}:{column}: {}", err.message)
            }
            Self::Read(..) | Self::Write(..) => self.to_string(),
            Self::Panic(path, err) => format!(
                "{path}: Panicked: {payload}",
                path = path_display(path.as_deref()),
                payload = err.payload
            ),
        }
    }
}

use miette::Diagnostic;
use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error, Diagnostic)]
pub enum Error {
    // -- Externals
    #[error(transparent)]
    #[diagnostic(code(djangofmt::io_error))]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic(code(djangofmt::serde_json_error))]
    SerdeJson(#[from] serde_json::Error),
}

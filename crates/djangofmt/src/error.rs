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
    #[diagnostic(code(djangofmt::miette_error))]
    Miette(#[from] miette::InstallError),

    #[error("{0}")]
    #[diagnostic(code(djangofmt::resolve_error))]
    Resolve(String),
}

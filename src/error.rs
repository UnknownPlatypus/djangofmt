use derive_more::{Display, From};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Display, From)]
#[display("{self:?}")]
pub enum Error {
    #[from]
    FormatCommand(Box<crate::commands::format::FormatCommandError>),
    // -- Externals
    #[from]
    Io(std::io::Error),

    #[from]
    SerdeJson(serde_json::Error),
}

// region:    --- Error Boilerplate

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate

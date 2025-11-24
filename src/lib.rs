use crate::args::Args;
use crate::logging::setup_tracing;
use std::process::ExitCode;

pub mod args;
pub mod commands;
pub mod error;
mod logging;

#[derive(Copy, Clone)]
pub enum ExitStatus {
    /// Command was successful and there were no errors.
    Success,
    /// Command was successful but there were errors.
    Failure,
    /// Command failed.
    Error,
}

impl From<ExitStatus> for ExitCode {
    fn from(status: ExitStatus) -> Self {
        match status {
            ExitStatus::Success => Self::from(0),
            ExitStatus::Failure => Self::from(1),
            ExitStatus::Error => Self::from(2),
        }
    }
}

/// Main entrypoint to any command.
/// Will set up logging and call the correct Command Handler.
///
/// # Errors
///
/// Will return `Err` on any formatting error (e.g. invalid file path, parse errors, formatting errors.).
pub fn run(
    Args {
        fmt,
        global_options,
        ..
    }: Args,
) -> error::Result<ExitStatus> {
    setup_tracing(global_options.log_level());

    commands::format::format(fmt, &global_options)
}

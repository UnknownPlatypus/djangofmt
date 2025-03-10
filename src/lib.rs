use std::process::ExitCode;

use anyhow::Result;

use crate::args::Args;
use crate::logging::setup_tracing;

pub mod args;
mod commands;
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
            ExitStatus::Success => ExitCode::from(0),
            ExitStatus::Failure => ExitCode::from(1),
            ExitStatus::Error => ExitCode::from(2),
        }
    }
}

/// Main entrypoint to any command.
/// Will set up logging and call the correct Command Handler.
pub fn run(
    Args {
        fmt,
        global_options,
        ..
    }: Args,
) -> Result<ExitStatus> {
    setup_tracing(global_options.log_level())?;

    commands::format::format(fmt, global_options)
}

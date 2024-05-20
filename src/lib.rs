use std::io::Write;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

use crate::args::{Args, Command};
use crate::logging::set_up_logging;

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
        command,
        global_options,
    }: Args,
) -> Result<ExitStatus> {
    set_up_logging(global_options.log_level())?;

    match command {
        Command::Format(args) => commands::format::format(args, global_options),
        Command::Version => {
            commands::version::version()?;
            Ok(ExitStatus::Success)
        }
    }
}

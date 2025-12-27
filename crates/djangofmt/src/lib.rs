use crate::args::Args;
use crate::logging::setup_tracing;
use clap::CommandFactory;
use std::process::ExitCode;
pub mod args;
pub mod commands;
pub mod error;
mod logging;
pub mod options;

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
        command,
    }: Args,
) -> error::Result<ExitStatus> {
    setup_tracing(global_options.log_level());

    match command {
        Some(args::Commands::Check(ref check_args)) => commands::check::check(check_args),
        Some(args::Commands::Completions { shell }) => {
            shell.generate(&mut Args::command(), &mut std::io::stdout());
            Ok(ExitStatus::Success)
        }
        None => commands::format::format(fmt),
    }
}

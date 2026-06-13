use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::CommandFactory;
use tracing::warn;

use crate::args::Args;
use crate::logging::setup_tracing;
pub mod args;
pub mod commands;
pub mod error;
pub mod fs;
pub mod line_width;
mod logging;
pub mod pyproject;
pub mod resolver;
#[cfg(test)]
mod test_support;

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
    setup_miette()?;

    match command {
        Some(args::Commands::Check(ref check_args)) => commands::check::check(check_args),
        Some(args::Commands::Completions { shell }) => {
            shell.generate(&mut Args::command(), &mut std::io::stdout());
            Ok(ExitStatus::Success)
        }
        None => {
            if is_stdin(&fmt.files, fmt.stdin_filename.as_deref()) {
                commands::format_stdin::format_stdin(&fmt)
            } else {
                commands::format::format(&fmt)
            }
        }
    }
}

/// Sentinel file argument meaning "read from standard input".
const STDIN_SENTINEL: &str = "-";

/// Returns true if the command should read from standard input.
fn is_stdin(files: &[PathBuf], stdin_filename: Option<&Path>) -> bool {
    let stdin_sentinel = Path::new(STDIN_SENTINEL);

    // If the user provided a `--stdin-filename`, always read from standard input.
    if stdin_filename.is_some() {
        if let Some(file) = files.iter().find(|file| file.as_path() != stdin_sentinel) {
            warn!(
                "Ignoring file {} in favor of standard input.",
                file.display()
            );
        }
        return true;
    }

    let [file] = files else {
        return false;
    };
    file == stdin_sentinel
}

fn setup_miette() -> error::Result<()> {
    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .show_related_errors_as_nested()
                .build(),
        )
    }))?;
    Ok(())
}

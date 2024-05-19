use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{command, Parser};

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

#[derive(Debug, Parser)]
#[command(
    author,
    name = "djangofmt",
    about = "Django Template Linter and Formatter",
    after_help = "For help with a specific command, see: `djangofmt help <command>`."
)]
#[command()]
pub struct Args {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    /// Run the formatter on the given files or directories.
    Format(FormatCommand),
}

#[derive(Clone, Debug, clap::Parser)]
#[allow(clippy::struct_excessive_bools)]
pub struct FormatCommand {
    /// List of files or directories to format.
    #[clap(help = "List of files or directories to format")]
    pub files: Vec<PathBuf>,
    /// Set the line-length.
    #[arg(long, help_heading = "Format configuration")]
    pub line_length: Option<u8>,
}

pub fn run(Args { command }: Args) -> Result<ExitStatus> {
    println!("{:?}", command);
    Ok(ExitStatus::Success)
}

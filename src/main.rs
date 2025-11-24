use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use colored::Colorize;

use djangofmt::args::{Args, Commands};
use djangofmt::{ExitStatus, run};

#[must_use]
pub fn main() -> ExitCode {
    let args = Args::parse();

    if let Some(Commands::Completions { shell }) = args.command {
        shell.generate(&mut Args::command(), &mut std::io::stdout());
    }

    match run(args) {
        Ok(exit_status) => exit_status.into(),
        Err(err) => {
            #[allow(clippy::print_stderr)]
            {
                // Unhandled error from djangofmt.
                eprintln!("{}", "djangofmt failed".red().bold());
                eprintln!("  {} {err}", "Error:".bold());
            }
            ExitStatus::Error.into()
        }
    }
}

use std::process::ExitCode;

use clap::Parser;
use colored::Colorize;

use djangofmt::args::Args;
use djangofmt::{run, ExitStatus};

pub fn main() -> ExitCode {
    let args = std::env::args_os();

    let args = Args::parse_from(args);

    match run(args) {
        Ok(code) => code.into(),
        Err(err) => {
            #[allow(clippy::print_stderr)]
            {
                // Unhandled error from djangofmt.
                eprintln!("{}", "djangofmt failed".red().bold());
                for cause in err.chain() {
                    eprintln!("  {} {cause}", "Cause:".bold());
                }
            }
            ExitStatus::Error.into()
        }
    }
}

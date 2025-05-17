use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use colored::Colorize;

use djangofmt::args::{Args, Commands};
use djangofmt::{ExitStatus, run};

pub fn main() -> ExitCode {
    let args = Args::parse();

    if let Some(Commands::Completions { shell }) = args.command {
        shell.generate(&mut Args::command(), &mut std::io::stdout());
    }

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

use std::path::PathBuf;

use clap::Parser;

use crate::logging::LogLevel;

/// All configuration options that can be passed "globally",
/// i.e., can be passed to all subcommands
#[derive(Debug, Default, Clone, clap::Args)]
pub struct GlobalConfigArgs {
    #[clap(flatten)]
    log_level_args: LogLevelArgs,
}

impl GlobalConfigArgs {
    pub fn log_level(&self) -> LogLevel {
        LogLevel::from(&self.log_level_args)
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
    #[clap(flatten)]
    pub(crate) global_options: GlobalConfigArgs,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    /// Run the formatter on the given files or directories.
    Format(FormatCommand),
    /// Display Djangofmt's version
    #[clap(alias = "--version")]
    Version,
}

#[derive(Clone, Debug, clap::Parser)]
#[allow(clippy::struct_excessive_bools)]
pub struct FormatCommand {
    /// List of files or directories to format.
    #[clap(help = "List of files or directories to format")]
    pub files: Vec<PathBuf>,
    /// Set the line-length.
    #[arg(long, help_heading = "Format configuration")]
    pub line_length: Option<usize>,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Default, Clone, clap::Args)]
pub struct LogLevelArgs {
    /// Enable verbose logging.
    #[arg(
        short,
        long,
        global = true,
        group = "verbosity",
        help_heading = "Log levels"
    )]
    pub verbose: bool,
    /// Print diagnostics, but nothing else.
    #[arg(
        short,
        long,
        global = true,
        group = "verbosity",
        help_heading = "Log levels"
    )]
    pub quiet: bool,
    /// Disable all logging (but still exit with status code "1" upon detecting diagnostics).
    #[arg(
        short,
        long,
        global = true,
        group = "verbosity",
        help_heading = "Log levels"
    )]
    pub silent: bool,
}

impl From<&LogLevelArgs> for LogLevel {
    fn from(args: &LogLevelArgs) -> Self {
        if args.silent {
            Self::Silent
        } else if args.quiet {
            Self::Quiet
        } else if args.verbose {
            Self::Verbose
        } else {
            Self::Default
        }
    }
}

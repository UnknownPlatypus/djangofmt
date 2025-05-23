use crate::logging::LogLevel;
use clap::builder::Styles;
use clap::builder::styling::{AnsiColor, Effects};
use std::path::PathBuf;

use markup_fmt::Language;

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

// Configures Clap v3-style help menu colors
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Debug, clap::Parser)]
#[command(
    author,
    version,
    next_line_help = true,
    about,
    styles=STYLES,
    subcommand_negates_reqs = true
)]
pub struct Args {
    #[clap(flatten)]
    pub(crate) fmt: FormatCommand,

    #[clap(flatten)]
    pub(crate) global_options: GlobalConfigArgs,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Clone, Debug, clap::Parser)]
pub struct FormatCommand {
    /// List of files to format.
    #[arg(help = "List of files to format", required = true)]
    pub files: Vec<PathBuf>,
    /// Set the line-length.
    #[arg(long, default_value = "120")]
    pub line_length: usize,
    /// Set the indent width.
    #[arg(long, default_value = "4")]
    pub indent_width: usize,
    /// Template language profile to use
    #[arg(long, value_enum, default_value = "django")]
    pub profile: Profile,
    /// Comma-separated list of custom block name to enable
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = clap::builder::ValueParser::new(|s: &str| Ok::<String, String>(s.trim().to_string())),
        value_name = "BLOCK_NAMES",
    )]
    pub custom_blocks: Option<Vec<String>>,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum Profile {
    Django,
    Jinja,
}

impl From<&Profile> for Language {
    fn from(profile: &Profile) -> Self {
        match profile {
            Profile::Django => Language::Django,
            Profile::Jinja => Language::Jinja,
        }
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum Commands {
    /// Generate shell completions
    #[clap(hide = true)]
    Completions {
        /// The shell to generate the completions for
        #[arg(value_enum)]
        shell: clap_complete_command::Shell,
    },
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
    /// Disable all logging.
    #[arg(
        short,
        long,
        global = true,
        group = "verbosity",
        help_heading = "Log levels"
    )]
    pub quiet: bool,
}

impl From<&LogLevelArgs> for LogLevel {
    fn from(args: &LogLevelArgs) -> Self {
        if args.quiet {
            Self::Quiet
        } else if args.verbose {
            Self::Verbose
        } else {
            Self::Default
        }
    }
}

#[cfg(test)]
mod tests {
    use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
    use std::process::Command;

    fn cli() -> Command {
        Command::new(get_cargo_bin("djangofmt"))
    }

    #[test]
    fn test_cli_help() {
        assert_cmd_snapshot!(cli().arg("--help"), @r###"
        success: true
        exit_code: 0
        ----- stdout -----
        A fast, HTML aware, Django template formatter, written in Rust.

        Usage: djangofmt [OPTIONS] <FILES>...

        Arguments:
          <FILES>...
                  List of files to format

        Options:
              --line-length <LINE_LENGTH>
                  Set the line-length [default: 120]
              --indent-width <INDENT_WIDTH>
                  Set the indent width [default: 4]
              --profile <PROFILE>
                  Template language profile to use [default: django] [possible values: django, jinja]
              --custom-blocks <BLOCK_NAMES>
                  Comma-separated list of custom block name to enable
          -h, --help
                  Print help
          -V, --version
                  Print version

        Log levels:
          -v, --verbose
                  Enable verbose logging
          -q, --quiet
                  Disable all logging

        ----- stderr -----
        "###);
    }
    #[test]
    fn test_cli_version() {
        assert_cmd_snapshot!(cli().arg("--version"), @r###"
        success: true
        exit_code: 0
        ----- stdout -----
        djangofmt 0.2.0

        ----- stderr -----
        "###);
    }
}

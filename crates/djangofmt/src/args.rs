use crate::line_width::{IndentWidth, LineLength, SelfClosing};
use crate::logging::LogLevel;
use clap::builder::Styles;
use clap::builder::styling::{AnsiColor, Effects};
use serde::Deserialize;
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
    #[must_use]
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

/// CLI arguments for File selection behavior.
#[derive(Clone, Debug, Default, clap::Args)]
#[command(next_help_heading = "File selection")]
#[expect(clippy::struct_excessive_bools)]
pub struct FileSelectionArgs {
    /// List of file path patterns to exclude. If provided, replaces the default excludes.
    #[arg(long, value_delimiter = ',', value_name = "FILE_PATTERN")]
    pub exclude: Option<Vec<String>>,
    /// List of file path patterns to add to the excluded paths.
    #[arg(long, value_delimiter = ',', value_name = "FILE_PATTERN")]
    pub extend_exclude: Option<Vec<String>>,

    /// Respect `.gitignore` files when discovering files. Use `--no-respect-gitignore` to disable.
    #[arg(long, overrides_with("no_respect_gitignore"))]
    pub respect_gitignore: bool,
    /// Do not respect .gitignore files when discovering files.
    #[arg(long, overrides_with("respect_gitignore"), hide = true)]
    pub no_respect_gitignore: bool,

    /// Enforce exclusions, even for paths passed to djangofmt directly on the command-line.
    /// Use `--no-force-exclude` to disable.
    #[arg(long, overrides_with("no_force_exclude"))]
    pub force_exclude: bool,
    #[clap(long, overrides_with("force_exclude"), hide = true)]
    pub no_force_exclude: bool,
}

#[derive(Clone, Debug, clap::Parser)]
pub struct FormatCommand {
    /// List of files or directories to format.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
    /// Set the line-length [default: 120]
    #[arg(long)]
    pub line_length: Option<LineLength>,
    /// Set the indent width [default: 4]
    #[arg(long)]
    pub indent_width: Option<IndentWidth>,
    /// Template language profile to use [default: django]
    #[arg(long, value_enum)]
    pub profile: Option<Profile>,
    /// Comma-separated list of custom block name to enable
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = clap::builder::ValueParser::new(|s: &str| Ok::<String, String>(s.trim().to_string())),
        value_name = "BLOCK_NAMES",
    )]
    pub custom_blocks: Option<Vec<String>>,
    /// Self-closing style for void HTML elements (e.g. <br> vs <br />) [default: never]
    #[arg(long, value_enum)]
    pub html_void_self_closing: Option<SelfClosing>,
    #[clap(flatten)]
    pub file_selection: FileSelectionArgs,
}

#[derive(Copy, Clone, Debug, clap::ValueEnum, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    #[default]
    Django,
    Jinja,
}

impl Profile {
    /// Infer the profile from a file's extension.
    ///
    /// - `.html` → `Django`
    /// - `.jinja`, `.jinja2`, `.j2` → `Jinja`
    ///
    /// Returns `None` for unrecognised extensions.
    #[must_use]
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "html" => Some(Self::Django),
            "jinja" | "jinja2" | "j2" => Some(Self::Jinja),
            _ => None,
        }
    }
}

impl From<Profile> for Language {
    fn from(profile: Profile) -> Self {
        match profile {
            Profile::Django => Self::Django,
            Profile::Jinja => Self::Jinja,
        }
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum Commands {
    /// Check files for lint errors
    #[clap(hide = true)]
    Check(CheckCommand),
    /// Generate shell completions
    #[clap(hide = true)]
    Completions {
        /// The shell to generate the completions for
        #[arg(value_enum)]
        shell: clap_complete_command::Shell,
    },
}

#[derive(Clone, Debug, clap::Parser)]
pub struct CheckCommand {
    /// List of files or directories to check.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
    /// Template language profile to use [default: django]
    #[arg(long, value_enum)]
    pub profile: Option<Profile>,
    #[clap(flatten)]
    pub file_selection: FileSelectionArgs,
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
        assert_cmd_snapshot!(cli().arg("--help"), @r#"
        success: true
        exit_code: 0
        ----- stdout -----
        A fast, HTML aware, Django template formatter, written in Rust.

        Usage: djangofmt [OPTIONS] <FILES>...

        Arguments:
          <FILES>...
                  List of files or directories to format

        Options:
              --line-length <LINE_LENGTH>
                  Set the line-length [default: 120]

              --indent-width <INDENT_WIDTH>
                  Set the indent width [default: 4]

              --profile <PROFILE>
                  Template language profile to use [default: django]
                  
                  [possible values: django, jinja]

              --custom-blocks <BLOCK_NAMES>
                  Comma-separated list of custom block name to enable

              --html-void-self-closing <HTML_VOID_SELF_CLOSING>
                  Self-closing style for void HTML elements (e.g. <br> vs <br />) [default: never]

                  Possible values:
                  - never:     Never use self-closing syntax
                  - always:    Always use self-closing syntax
                  - unchanged: Keep existing style as-is

          -h, --help
                  Print help (see a summary with '-h')

          -V, --version
                  Print version

        File selection:
              --exclude <FILE_PATTERN>
                  List of file path patterns to exclude. If provided, replaces the default excludes

              --extend-exclude <FILE_PATTERN>
                  List of file path patterns to add to the excluded paths

              --respect-gitignore
                  Respect `.gitignore` files when discovering files. Use `--no-respect-gitignore` to disable

              --force-exclude
                  Enforce exclusions, even for paths passed to djangofmt directly on the command-line. Use
                  `--no-force-exclude` to disable

        Log levels:
          -v, --verbose
                  Enable verbose logging

          -q, --quiet
                  Disable all logging

        ----- stderr -----
        "#);
    }
    #[test]
    fn test_cli_version() {
        assert_cmd_snapshot!(cli().arg("--version"), @r###"
        success: true
        exit_code: 0
        ----- stdout -----
        djangofmt 0.2.6

        ----- stderr -----
        "###);
    }

    #[test]
    fn test_cli_invalid_line_length() {
        assert_cmd_snapshot!(cli().args(["--line-length", "321", "test.html"]), @r###"
        success: false
        exit_code: 2
        ----- stdout -----

        ----- stderr -----
        error: invalid value '321' for '--line-length <LINE_LENGTH>': line-length must be between 1 and 320 (got 321)

        For more information, try '--help'.
        "###);
    }

    #[test]
    fn test_cli_invalid_indent_width() {
        assert_cmd_snapshot!(cli().args(["--indent-width", "17", "test.html"]), @r###"
        success: false
        exit_code: 2
        ----- stdout -----

        ----- stderr -----
        error: invalid value '17' for '--indent-width <INDENT_WIDTH>': indent-width must be between 1 and 16 (got 17)

        For more information, try '--help'.
        "###);
    }
}

use crate::logging::LogLevel;
use clap::builder::Styles;
use clap::builder::styling::{AnsiColor, Effects};
use djangofmt_formatter::line_width::{IndentWidth, LineLength, SelfClosing};
use serde::Deserialize;
use std::path::PathBuf;

use djangofmt_lint::{DjangoVersion, RuleSelector};
use djangofmt_syntax::Profile;

/// All configuration options that can be passed "globally",
/// i.e., can be passed to all subcommands.
#[derive(Debug, Default, Clone, clap::Args)]
pub struct GlobalConfigArgs {
    #[clap(flatten)]
    log_level_args: LogLevelArgs,
}

impl GlobalConfigArgs {
    #[must_use]
    pub(crate) fn log_level(&self) -> LogLevel {
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
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is a distinct user-facing CLI flag"
)]
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

/// CLI arguments describing the templates being processed, shared by `format` and `check`.
#[derive(Clone, Debug, Default, clap::Args)]
pub struct TemplateArgs {
    /// Template language profile to use [default: django].
    #[arg(long, value_enum)]
    pub profile: Option<Profile>,
    /// Comma-separated list of custom block name to enable.
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = clap::builder::ValueParser::new(|s: &str| Ok::<String, String>(s.trim().to_string())),
        value_name = "BLOCK_NAMES",
    )]
    pub custom_blocks: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, clap::Parser)]
pub struct FormatCommand {
    /// List of files or directories to format.
    #[arg(required_unless_present = "stdin_filename")]
    pub files: Vec<PathBuf>,
    /// The name of the file when passing it through stdin.
    #[arg(long, value_name = "PATH")]
    pub stdin_filename: Option<PathBuf>,
    /// Set the line-length [default: 120].
    #[arg(long)]
    pub line_length: Option<LineLength>,
    /// Set the indent width [default: 4].
    #[arg(long)]
    pub indent_width: Option<IndentWidth>,
    #[clap(flatten)]
    pub template: TemplateArgs,
    /// Self-closing style for void HTML elements (e.g. <br> vs <br />) [default: never].
    #[arg(long, value_enum)]
    pub html_void_self_closing: Option<SelfClosing>,
    /// Avoid writing any formatted files back; instead, exit with a non-zero status code if any
    /// files would have been modified, and zero otherwise.
    #[arg(long)]
    pub check: bool,
    /// Deprecated, removed in djangofmt 2.0: preserve unquoted HTML attribute values.
    #[arg(long, overrides_with("no_preserve_unquoted_attrs"), hide = true)]
    pub preserve_unquoted_attrs: bool,
    /// Do not preserve unquoted HTML attribute values.
    #[arg(long, overrides_with("preserve_unquoted_attrs"), hide = true)]
    pub no_preserve_unquoted_attrs: bool,
    #[clap(flatten)]
    pub file_selection: FileSelectionArgs,
}

#[derive(Debug, clap::Subcommand)]
pub enum Commands {
    /// Check files for lint errors.
    Check(Box<CheckCommand>),
    /// Generate shell completions.
    #[clap(hide = true)]
    Completions {
        /// The shell to generate the completions for.
        #[arg(value_enum)]
        shell: clap_complete_command::Shell,
    },
}

/// How `check` renders diagnostics.
#[derive(Copy, Clone, Debug, clap::ValueEnum, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    /// Rich diagnostics with source snippets and help text.
    #[default]
    Full,
    /// One line per diagnostic: `path:line:col: rule [*] message`.
    Concise,
}

/// CLI arguments for selecting which lint rules run.
#[derive(Clone, Debug, Default, clap::Args)]
#[command(next_help_heading = "Rule selection")]
pub struct RuleSelectionArgs {
    /// Comma-separated list of rules or categories to enable (e.g. `category:default`, `category:all`, `category:correctness`, `missing-img-alt`).
    #[arg(long, value_delimiter = ',', value_parser = parse_rule_selector, value_name = "RULE")]
    pub select: Option<Vec<RuleSelector>>,
    /// Comma-separated list of rules or categories to disable.
    #[arg(long, value_delimiter = ',', value_parser = parse_rule_selector, value_name = "RULE")]
    pub ignore: Option<Vec<RuleSelector>>,
    /// Enable preview rules. Use `--no-preview` to disable.
    #[arg(long, overrides_with("no_preview"))]
    pub preview: bool,
    #[arg(long, overrides_with("preview"), hide = true)]
    pub no_preview: bool,
}

/// Parse a single rule selector, surfacing the grammar error as a clap error.
fn parse_rule_selector(value: &str) -> Result<RuleSelector, String> {
    value
        .trim()
        .parse::<RuleSelector>()
        .map_err(|err| err.to_string())
}

#[derive(Clone, Debug, Default, clap::Parser)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is a distinct user-facing CLI flag"
)]
pub struct CheckCommand {
    /// List of files or directories to check.
    #[arg(required_unless_present = "stdin_filename")]
    pub files: Vec<PathBuf>,
    /// The name of the file when passing it through stdin.
    #[arg(long, value_name = "PATH")]
    pub stdin_filename: Option<PathBuf>,
    #[clap(flatten)]
    pub template: TemplateArgs,
    /// Apply safe fixes automatically. Use `--no-fix` to disable.
    #[arg(long, overrides_with("no_fix"))]
    pub fix: bool,
    #[arg(long, overrides_with("fix"), hide = true)]
    pub no_fix: bool,
    /// Include unsafe fixes when applying with --fix. Without --fix,
    /// diagnostics from unsafe fixes are still reported as fixable.
    /// Use `--no-unsafe-fixes` to disable.
    #[arg(long, overrides_with("no_unsafe_fixes"))]
    pub unsafe_fixes: bool,
    #[arg(long, overrides_with("unsafe_fixes"), hide = true)]
    pub no_unsafe_fixes: bool,
    /// Output format for diagnostics [default: full].
    #[arg(long, value_enum)]
    pub output_format: Option<OutputFormat>,
    /// List per-rule fix counts after applying. Use `--no-show-fixes` to disable.
    #[arg(long, overrides_with("no_show_fixes"))]
    pub show_fixes: bool,
    #[arg(long, overrides_with("show_fixes"), hide = true)]
    pub no_show_fixes: bool,
    /// The Django version the templates target, as major.minor (e.g. 5.2).
    #[arg(long, value_name = "VERSION")]
    pub target_version: Option<DjangoVersion>,
    #[clap(flatten)]
    pub rule_selection: RuleSelectionArgs,
    #[clap(flatten)]
    pub file_selection: FileSelectionArgs,
}

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

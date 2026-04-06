use rayon::iter::Either::{Left, Right};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::borrow::Cow;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, error, info};

use crate::ExitStatus;
use crate::args::{FormatCommand, Profile};
use crate::error::{CommandError, ParseError, Result};
use crate::line_width::{IndentWidth, LineLength, SelfClosing};
use crate::pyproject::PyprojectSettings;

/// Pre-built configuration for all formatters.
pub struct FormatterConfig {
    /// Config for main HTML/Jinja formatter
    pub markup: markup_fmt::config::FormatOptions,
    /// Config for CSS/SCSS formatter
    pub malva: malva::config::FormatOptions,
    /// Config for JSON formatter
    pub json: dprint_plugin_json::configuration::Configuration,
}

impl FormatterConfig {
    #[must_use]
    pub fn new(
        print_width: LineLength,
        indent_width: IndentWidth,
        custom_blocks: Option<Vec<String>>,
        html_void_self_closing: SelfClosing,
    ) -> Self {
        Self {
            markup: build_markup_options(
                print_width,
                indent_width,
                custom_blocks,
                html_void_self_closing,
            ),
            malva: build_malva_config(print_width, indent_width),
            json: build_json_config(print_width, indent_width),
        }
    }

    /// Build a [`FormatterConfig`] by merging CLI arguments with pyproject.toml settings.
    ///
    /// CLI arguments take precedence over pyproject settings, which take precedence over defaults.
    #[must_use]
    pub fn from_args(args: &FormatCommand, pyproject: &PyprojectSettings) -> Self {
        let line_length = args
            .line_length
            .or(pyproject.line_length)
            .unwrap_or_default();
        let indent_width = args
            .indent_width
            .or(pyproject.indent_width)
            .unwrap_or_default();
        let custom_blocks =
            merge_custom_blocks(args.custom_blocks.clone(), pyproject.custom_blocks.clone());
        let html_void_self_closing = args
            .html_void_self_closing
            .or(pyproject.html_void_self_closing)
            .unwrap_or_default();

        Self::new(
            line_length,
            indent_width,
            custom_blocks,
            html_void_self_closing,
        )
    }
}

/// Merge custom blocks from CLI arguments and pyproject.toml settings, deduplicating entries.
fn merge_custom_blocks(
    cli: Option<Vec<String>>,
    pyproject: Option<Vec<String>>,
) -> Option<Vec<String>> {
    let mut merged: Vec<String> = cli.into_iter().chain(pyproject).flatten().collect();

    if merged.is_empty() {
        None
    } else {
        merged.sort_unstable();
        merged.dedup();
        Some(merged)
    }
}

macro_rules! ignore_directive {
    () => {
        "djangofmt:ignore"
    };
}
const DJANGOFMT_IGNORE_COMMENT_DIRECTIVE: &str = ignore_directive!();
const DJANGOFMT_IGNORE_COMMENT: &str = concat!("<!-- ", ignore_directive!(), " -->");

/// Build default `markup_fmt` options for HTML/Jinja formatting.
#[must_use]
pub fn build_markup_options(
    print_width: LineLength,
    indent_width: IndentWidth,
    custom_blocks: Option<Vec<String>>,
    html_void_self_closing: SelfClosing,
) -> markup_fmt::config::FormatOptions {
    markup_fmt::config::FormatOptions {
        layout: markup_fmt::config::LayoutOptions {
            print_width: print_width.into(),
            indent_width: indent_width.into(),
            ..markup_fmt::config::LayoutOptions::default()
        },
        language: markup_fmt::config::LanguageOptions {
            format_comments: false,
            // HTML void elements should not be self-closing by default:
            // See https://developer.mozilla.org/en-US/docs/Glossary/Void_element#self-closing_tags
            // <br/> -> <br>
            html_void_self_closing: html_void_self_closing.into(),
            // SVG elements should be self-closing:
            // <circle cx="50" cy="50" r="50"></circle> -> <circle cx="50" cy="50" r="50" />
            svg_self_closing: Some(true),
            // MathML elements should be self-closing:
            // <mspace width="1em"></mspace> -> <mspace width="1em" />
            mathml_self_closing: Some(true),
            // HTML normal elements should not be self-closing:
            // <div/> -> <div></div>
            // <div/>desfsdf -> <div></div>desfsdf
            // TODO: This is actually slightly incorrect (but better than nothing).
            //       We need a parse error or to match browser recovery to <div>desfsdf</div>
            html_normal_self_closing: Some(false),
            // This is actually nice to keep this setting false, it makes it possible to control wrapping
            // of props semi manually by inserting or not a newline before the first prop.
            // See https://github.com/g-plane/markup_fmt/issues/10 that showcase this.
            // <div
            //     class="foo"
            //     id="bar">
            // </div>
            prefer_attrs_single_line: false,
            // Parse custom Django template blocks:
            // For ex "stage,cache,flatblock,section,csp_compress"
            // {% stage %}...{% endstage %}
            // {% cache %}...{% endcache %}
            custom_blocks,
            // Ignore formatting with comment directive:
            // <!-- djangofmt:ignore -->
            // <div>unformatted</div>
            ignore_comment_directive: DJANGOFMT_IGNORE_COMMENT_DIRECTIVE.into(),
            ignore_file_comment_directive: DJANGOFMT_IGNORE_COMMENT_DIRECTIVE.into(),
            // Indent style tags content:
            // <style>
            //     body { color: red }
            // </style>
            style_indent: true,
            // Indent script tags content:
            // <script>
            //     console.log("hello");
            // </script>
            script_indent: true,
            ..markup_fmt::config::LanguageOptions::default()
        },
    }
}

/// Build default `malva` options for CSS/SCSS/SASS/LESS formatting.
fn build_malva_config(
    print_width: LineLength,
    indent_width: IndentWidth,
) -> malva::config::FormatOptions {
    malva::config::FormatOptions {
        layout: malva::config::LayoutOptions {
            print_width: print_width.into(),
            indent_width: indent_width.into(),
            ..malva::config::LayoutOptions::default()
        },
        language: malva::config::LanguageOptions {
            // Because markup_fmt uses DoubleQuotes
            quotes: malva::config::Quotes::AlwaysSingle,
            operator_linebreak: malva::config::OperatorLineBreak::Before,
            format_comments: true,
            linebreak_in_pseudo_parens: true,
            declaration_order: Some(malva::config::DeclarationOrder::Smacss),
            keyframe_selector_notation: Some(malva::config::KeyframeSelectorNotation::Percentage),
            single_line_top_level_declarations: true,
            selector_override_comment_directive: "djangofmt-selector-override".into(),
            ignore_comment_directive: DJANGOFMT_IGNORE_COMMENT_DIRECTIVE.into(),
            ignore_file_comment_directive: DJANGOFMT_IGNORE_COMMENT_DIRECTIVE.into(),
            ..malva::config::LanguageOptions::default()
        },
    }
}

fn build_json_config(
    print_width: LineLength,
    indent_width: IndentWidth,
) -> dprint_plugin_json::configuration::Configuration {
    dprint_plugin_json::configuration::ConfigurationBuilder::new()
        .line_width(print_width.value().into())
        .indent_width(indent_width.value())
        .build()
}

pub fn format(args: &FormatCommand) -> Result<ExitStatus> {
    let resolved = super::resolve_command(&args.files, args.profile, &args.file_selection)?;
    let config = FormatterConfig::from_args(args, &resolved.pyproject);

    // Format files in parallel
    let start = Instant::now();
    let (results, mut parse_errors): (Vec<_>, Vec<_>) = resolved
        .files
        .par_iter()
        .map(|entry| format_path(entry, &config, resolved.profile))
        .partition_map(|result| match result {
            Ok(fmt_res) => Left(fmt_res),
            Err(err) => Right(err),
        });

    debug!(
        "Formatted {} files in {:.2?}",
        resolved.files.len(),
        start.elapsed()
    );

    // Report on any parsing errors.
    parse_errors.sort_unstable_by(|a, b| a.path().cmp(&b.path()));
    let nb_errors = parse_errors.len();
    for error in parse_errors {
        error!("{:?}", miette::Report::new(*error));
    }
    if nb_errors > 0 {
        error!("Couldn't format {nb_errors} files!");
    }

    // Report on the formatting changes.
    let summary = build_summary(results.as_ref());
    if !summary.is_empty() {
        info!("{} !", summary);
    }

    if nb_errors == 0 {
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::Failure)
    }
}

/// Format the given source code.
pub fn format_text(
    source: &str,
    config: &FormatterConfig,
    profile: Profile,
) -> std::result::Result<Option<String>, markup_fmt::FormatError<crate::error::Error>> {
    if source.starts_with(DJANGOFMT_IGNORE_COMMENT) {
        return Ok(None);
    }
    markup_fmt::format_text(
        source,
        markup_fmt::Language::from(profile),
        &config.markup,
        |code, hints| {
            match hints.ext {
                "json" | "jsonc" => {
                    let fake_filename = PathBuf::from(format!("djangofmt_fmt_stdin.{}", hints.ext));
                    let mut json_config = config.json.clone();
                    json_config.line_width = u32::try_from(hints.print_width).unwrap_or(u32::MAX);
                    match dprint_plugin_json::format_text(&fake_filename, code, &json_config) {
                        Ok(Some(formatted)) => Ok(formatted.into()),
                        Ok(None) => Ok(code.into()),
                        Err(error) => {
                            debug!(
                                "Failed to format JSON, falling back to original code. Error: {:?}",
                                error
                            );
                            Ok(code.into())
                        }
                    }
                }
                "css" | "scss" | "sass" | "less" => {
                    let mut malva_config = config.malva.clone();
                    malva_config.layout.print_width = hints.print_width;

                    let formatted_css = malva::format_text(code, malva::Syntax::Css, &malva_config)
                        .map_or_else(
                            |error| {
                                debug!(
                                    "Failed to format CSS, falling back to original code. Error: {:?}",
                                    error
                                );
                                code.into()
                            },
                            Cow::from,
                        );

                    // Workaround a bug in malva -> https://github.com/g-plane/malva/issues/44
                    // Tries to keep on formatting style attr on a single line like expected with
                    // single_line_top_level_declarations = true
                    if code.contains('{') {
                        Ok(formatted_css)
                    } else {
                        Ok(formatted_css
                            .lines()
                            .map(str::trim)
                            .collect::<Vec<_>>()
                            .join(" ")
                            .into())
                    }
                }
                _ => Ok(code.into()),
            }
        },
    )
    .map(Some)
}

/// Format the file at the given [`Path`].
#[tracing::instrument(level="debug", skip_all, fields(path = %path.display()))]
fn format_path(
    path: &Path,
    config: &FormatterConfig,
    profile: Option<Profile>,
) -> std::result::Result<FormatResult, Box<CommandError>> {
    let profile = profile
        .or_else(|| Profile::from_path(path))
        .unwrap_or_default();
    let unformatted = std::fs::read_to_string(path)
        .map_err(|err| CommandError::Read(Some(path.to_path_buf()), err))?;

    let formatted = match format_text(&unformatted, config, profile) {
        Ok(f) => f,
        Err(err) => {
            return Err(Box::new(CommandError::Parse(ParseError::new(
                Some(path.to_path_buf()),
                unformatted,
                &err,
            ))));
        }
    };

    let Some(formatted) = formatted else {
        return Ok(FormatResult::Skipped);
    };

    // Checked if something changed and write to file if necessary
    if formatted == unformatted {
        Ok(FormatResult::Unchanged)
    } else {
        let mut writer =
            File::create(path).map_err(|err| CommandError::Write(Some(path.to_path_buf()), err))?;

        writer
            .write_all(formatted.as_bytes())
            .map_err(|err| CommandError::Write(Some(path.to_path_buf()), err))?;

        Ok(FormatResult::Formatted)
    }
}

/// The result of an individual formatting operation.
#[derive(Eq, PartialEq, Debug)]
pub enum FormatResult {
    /// The file was formatted.
    Formatted,

    /// The file was unchanged, as the formatted contents matched the existing contents.
    Unchanged,

    /// The file was skipped due to a top-level ignore comment.
    Skipped,
}

/// Write a summary of the formatting results to stdout.
#[must_use]
pub fn build_summary(results: &[FormatResult]) -> String {
    let (mut changed, mut unchanged, mut skipped) = (0usize, 0usize, 0usize);
    for result in results {
        match result {
            FormatResult::Formatted => changed += 1,
            FormatResult::Unchanged => unchanged += 1,
            FormatResult::Skipped => skipped += 1,
        }
    }

    let parts: Vec<String> = [
        (changed, "reformatted"),
        (unchanged, "left unchanged"),
        (skipped, "skipped"),
    ]
    .iter()
    .filter(|(count, _)| *count > 0)
    .map(|(count, label)| {
        format!(
            "{} file{} {}",
            count,
            if *count == 1 { "" } else { "s" },
            label
        )
    })
    .collect();

    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    use rstest::rstest;
    #[test]
    fn format_command_error_read_display() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err = CommandError::Read(Some(PathBuf::from("/path/to/file.html")), io_err);
        assert_eq!(
            err.to_string(),
            "Failed to read /path/to/file.html: file not found"
        );
    }

    #[test]
    fn format_command_error_read_display_unknown_path() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        let err = CommandError::Read(None, io_err);
        assert_eq!(
            err.to_string(),
            "Failed to read <unknown>: permission denied"
        );
    }

    #[test]
    fn format_command_error_write_display() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        let err = CommandError::Write(Some(PathBuf::from("/path/to/output.html")), io_err);
        assert_eq!(
            err.to_string(),
            "Failed to write /path/to/output.html: permission denied"
        );
    }

    #[test]
    fn format_command_error_write_display_unknown_path() {
        let io_err = io::Error::other("disk full");
        let err = CommandError::Write(None, io_err);
        assert_eq!(err.to_string(), "Failed to write <unknown>: disk full");
    }

    #[test]
    fn merge_custom_blocks_both_none() {
        assert_eq!(merge_custom_blocks(None, None), None);
    }

    #[test]
    fn merge_custom_blocks_only_cli() {
        let mut result = merge_custom_blocks(Some(vec!["foo".into(), "bar".into()]), None).unwrap();
        result.sort();
        assert_eq!(result, vec!["bar", "foo"]);
    }

    #[test]
    fn merge_custom_blocks_only_pyproject() {
        assert_eq!(
            merge_custom_blocks(None, Some(vec!["baz".into()])),
            Some(vec!["baz".to_string()])
        );
    }

    #[test]
    fn merge_custom_blocks_both_without_overlap() {
        let mut result =
            merge_custom_blocks(Some(vec!["foo".into()]), Some(vec!["bar".into()])).unwrap();
        result.sort();
        assert_eq!(result, vec!["bar", "foo"]);
    }

    #[test]
    fn merge_custom_blocks_both_with_duplicates() {
        let mut result = merge_custom_blocks(
            Some(vec!["foo".into(), "bar".into()]),
            Some(vec!["bar".into(), "baz".into()]),
        )
        .unwrap();
        result.sort();
        assert_eq!(result, vec!["bar", "baz", "foo"]);
    }

    #[test]
    fn formatter_config_from_args_defaults() {
        let args = FormatCommand {
            files: vec![],
            line_length: None,
            indent_width: None,
            profile: None,
            custom_blocks: None,
            html_void_self_closing: None,
            file_selection: crate::args::FileSelectionArgs::default(),
        };
        let pyproject = PyprojectSettings::default();
        let config = FormatterConfig::from_args(&args, &pyproject);
        assert_eq!(config.markup.layout.print_width, 120);
        assert_eq!(config.markup.layout.indent_width, 4);
    }

    #[test]
    fn formatter_config_from_args_cli_overrides_pyproject() {
        let args = FormatCommand {
            files: vec![],
            line_length: Some(LineLength::try_from(80u16).unwrap()),
            indent_width: Some(IndentWidth::try_from(2u8).unwrap()),
            profile: None,
            custom_blocks: None,
            html_void_self_closing: Some(SelfClosing::Always),
            file_selection: crate::args::FileSelectionArgs::default(),
        };
        let pyproject = PyprojectSettings {
            line_length: Some(LineLength::try_from(200u16).unwrap()),
            indent_width: Some(IndentWidth::try_from(8u8).unwrap()),
            html_void_self_closing: Some(SelfClosing::Never),
            ..Default::default()
        };
        let config = FormatterConfig::from_args(&args, &pyproject);
        assert_eq!(config.markup.layout.print_width, 80);
        assert_eq!(config.markup.layout.indent_width, 2);
        assert_eq!(config.markup.language.html_void_self_closing, Some(true));
    }

    #[test]
    fn formatter_config_from_args_falls_back_to_pyproject() {
        let args = FormatCommand {
            files: vec![],
            line_length: None,
            indent_width: None,
            profile: None,
            custom_blocks: None,
            html_void_self_closing: None,
            file_selection: crate::args::FileSelectionArgs::default(),
        };
        let pyproject = PyprojectSettings {
            line_length: Some(LineLength::try_from(200u16).unwrap()),
            ..Default::default()
        };
        let config = FormatterConfig::from_args(&args, &pyproject);
        assert_eq!(config.markup.layout.print_width, 200);
    }

    #[rstest]
    #[case(vec![], "")]
    #[case(vec![FormatResult::Formatted], "1 file reformatted")]
    #[case(vec![FormatResult::Formatted, FormatResult::Formatted], "2 files reformatted")]
    #[case(vec![FormatResult::Unchanged], "1 file left unchanged")]
    #[case(vec![FormatResult::Unchanged, FormatResult::Unchanged], "2 files left unchanged")]
    #[case(vec![FormatResult::Skipped], "1 file skipped")]
    #[case(vec![FormatResult::Skipped, FormatResult::Skipped], "2 files skipped")]
    #[case(vec![FormatResult::Formatted, FormatResult::Unchanged], "1 file reformatted, 1 file left unchanged")]
    #[case(vec![FormatResult::Formatted, FormatResult::Formatted, FormatResult::Unchanged], "2 files reformatted, 1 file left unchanged")]
    #[case(vec![FormatResult::Formatted, FormatResult::Skipped], "1 file reformatted, 1 file skipped")]
    #[case(vec![FormatResult::Formatted, FormatResult::Skipped, FormatResult::Skipped], "1 file reformatted, 2 files skipped")]
    #[case(vec![FormatResult::Unchanged, FormatResult::Skipped], "1 file left unchanged, 1 file skipped")]
    #[case(vec![FormatResult::Unchanged, FormatResult::Unchanged, FormatResult::Skipped], "2 files left unchanged, 1 file skipped")]
    #[case(vec![FormatResult::Formatted, FormatResult::Unchanged, FormatResult::Skipped], "1 file reformatted, 1 file left unchanged, 1 file skipped")]
    #[case(vec![
        FormatResult::Formatted,
        FormatResult::Formatted,
        FormatResult::Unchanged,
        FormatResult::Skipped,
        FormatResult::Skipped,
        FormatResult::Skipped,
    ], "2 files reformatted, 1 file left unchanged, 3 files skipped")]
    fn test_write_summary(#[case] results: Vec<FormatResult>, #[case] expected: &str) {
        assert_eq!(build_summary(&results), expected);
    }
}

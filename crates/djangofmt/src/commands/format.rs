use djangofmt_formatter::{FormatterConfig, format_text};
use djangofmt_syntax::{ParseError, Profile};
use rayon::iter::Either::{Left, Right};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::borrow::Cow;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info, warn};

use crate::ExitStatus;
use crate::args::{FormatCommand, OutputFormat};
use crate::config::{resolve_bool_arg, resolve_profile};
use crate::editorconfig::{self, EditorconfigSettings};
use crate::error::{CommandError, Result};
use crate::fs::relativize_path;
use crate::pyproject::PyprojectSettings;
use editorconfig_parser::EditorConfig;

/// Build a [`FormatterConfig`] by merging CLI arguments with `pyproject.toml` and `.editorconfig` settings.
///
/// Precedence is `cli` > `pyproject` > `editorconfig` > `default`
#[must_use]
pub fn formatter_config_from_args(
    args: &FormatCommand,
    pyproject: &PyprojectSettings,
    editorconfig: &EditorconfigSettings,
) -> FormatterConfig {
    let line_length = args
        .line_length
        .or(pyproject.line_length)
        .or(editorconfig.line_length)
        .unwrap_or_default();
    let indent_width = args
        .indent_width
        .or(pyproject.indent_width)
        .or(editorconfig.indent_width)
        .unwrap_or_default();
    let custom_blocks = merge_custom_blocks(
        args.template.custom_blocks.clone(),
        pyproject.custom_blocks.clone(),
    );
    let html_void_self_closing = args
        .html_void_self_closing
        .or(pyproject.html_void_self_closing)
        .unwrap_or_default();
    let preserve_unquoted_attrs = resolve_bool_arg(
        args.preserve_unquoted_attrs,
        args.no_preserve_unquoted_attrs,
    )
    .or(pyproject.preserve_unquoted_attrs)
    .unwrap_or_default();

    FormatterConfig::new(
        line_length,
        indent_width,
        custom_blocks,
        html_void_self_closing,
        preserve_unquoted_attrs,
    )
}

/// Merge custom blocks from CLI arguments and pyproject.toml settings, deduplicating entries.
pub(crate) fn merge_custom_blocks(
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

/// Per-run inputs used to derive a per-file [`FormatterConfig`] and [`Profile`].
struct FormatContext<'a> {
    args: &'a FormatCommand,
    pyproject: &'a PyprojectSettings,
    editorconfig: Option<&'a EditorConfig>,
    /// Built once when `.editorconfig` can't vary per file (no config, or only `[*]`).
    config: Option<FormatterConfig>,
}

impl<'a> FormatContext<'a> {
    fn new(
        args: &'a FormatCommand,
        pyproject: &'a PyprojectSettings,
        editorconfig: Option<&'a EditorConfig>,
    ) -> Self {
        let mut context = Self {
            args,
            pyproject,
            editorconfig,
            config: None,
        };
        if !editorconfig::has_per_file_sections(editorconfig) {
            // Any filename resolves the same settings here, so build the config once.
            context.config = Some(context.build_config(Path::new("any")));
        }
        context
    }

    fn profile_for(&self, path: &Path) -> Profile {
        resolve_profile(
            self.args.template.profile,
            self.pyproject.profile,
            Some(path),
        )
    }

    /// The config for `path`: the shared one when set, otherwise built for this file.
    fn config_for(&self, path: &Path) -> Cow<'_, FormatterConfig> {
        self.config
            .as_ref()
            .map_or_else(|| Cow::Owned(self.build_config(path)), Cow::Borrowed)
    }

    fn build_config(&self, path: &Path) -> FormatterConfig {
        let editorconfig = editorconfig::resolve_editorconfig(self.editorconfig, path);
        formatter_config_from_args(self.args, self.pyproject, &editorconfig)
    }
}

/// Warn here, once per run, rather than in [`formatter_config_from_args`], which may run per file.
pub(crate) fn warn_deprecated_options(args: &FormatCommand, pyproject: &PyprojectSettings) {
    if args.preserve_unquoted_attrs
        || args.no_preserve_unquoted_attrs
        || pyproject.preserve_unquoted_attrs.is_some()
    {
        warn!(
            "`preserve-unquoted-attrs` is deprecated and will be removed in djangofmt 2.0. \
             Unquoted values on Cotton components (`c-*`) are now preserved automatically."
        );
    }
}

pub fn format(args: &FormatCommand) -> Result<ExitStatus> {
    let resolved = super::resolve_command(&args.files, &args.file_selection)?;
    warn_deprecated_options(args, &resolved.pyproject);
    let editorconfig = editorconfig::load_editorconfig_from_cwd();
    let context = FormatContext::new(args, &resolved.pyproject, editorconfig.as_ref());

    // Format files in parallel
    let start = Instant::now();
    let (results, errors): (Vec<_>, Vec<_>) = resolved
        .files
        .par_iter()
        .map(|entry| {
            super::catch_file_panic(Some(entry), || format_path(entry, &context))
                .map(|fmt_res| (entry.as_path(), fmt_res))
        })
        .partition_map(|result| match result {
            Ok(fmt_res) => Left(fmt_res),
            Err(err) => Right(*err),
        });

    debug!(
        "Formatted {} files in {:.2?}",
        resolved.files.len(),
        start.elapsed()
    );

    let nb_errors = super::report_errors(errors, "format", OutputFormat::Full);

    // In check mode, list what would change instead of writing it.
    // `resolved.files` is sorted and rayon keeps that order, so the listing is deterministic.
    let mut would_reformat = false;
    if args.check {
        for (path, res) in &results {
            if *res == FormatResult::Formatted {
                would_reformat = true;
                info!("Would reformat: {}", relativize_path(path));
            }
        }
    }

    // Report on the formatting changes.
    let summary = build_summary(results.iter().map(|(_, res)| res), args.check);
    if !summary.is_empty() {
        info!("{} !", summary);
    }

    if nb_errors > 0 {
        return Ok(ExitStatus::Error);
    }
    if would_reformat {
        return Ok(ExitStatus::Failure);
    }
    Ok(ExitStatus::Success)
}

/// Format the file at the given [`Path`].
#[tracing::instrument(level="debug", skip_all, fields(path = %path.display()))]
fn format_path(
    path: &Path,
    context: &FormatContext,
) -> std::result::Result<FormatResult, Box<CommandError>> {
    let profile = context.profile_for(path);
    let config = context.config_for(path);
    let unformatted = std::fs::read_to_string(path)
        .map_err(|err| CommandError::Read(Some(path.to_path_buf()), err))?;

    let formatted = match format_text(&unformatted, &config, profile, Some(path)) {
        Ok(f) => f,
        Err(err) => {
            let err = ParseError::new(relativize_path(path), unformatted, &err);
            return Err(Box::new(CommandError::Parse(Some(path.to_path_buf()), err)));
        }
    };

    let Some(formatted) = formatted else {
        return Ok(FormatResult::Skipped);
    };

    // Checked if something changed and write to file if necessary
    if formatted == unformatted {
        Ok(FormatResult::Unchanged)
    } else {
        if !context.args.check {
            let mut writer = File::create(path)
                .map_err(|err| CommandError::Write(Some(path.to_path_buf()), err))?;

            writer
                .write_all(formatted.as_bytes())
                .map_err(|err| CommandError::Write(Some(path.to_path_buf()), err))?;
        }

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
pub fn build_summary<'a>(
    results: impl IntoIterator<Item = &'a FormatResult>,
    check: bool,
) -> String {
    let (mut changed, mut unchanged, mut skipped) = (0usize, 0usize, 0usize);
    for result in results {
        match result {
            FormatResult::Formatted => changed += 1,
            FormatResult::Unchanged => unchanged += 1,
            FormatResult::Skipped => skipped += 1,
        }
    }

    let (changed_label, unchanged_label) = if check {
        ("would be reformatted", "already formatted")
    } else {
        ("reformatted", "left unchanged")
    };
    let parts: Vec<String> = [
        (changed, changed_label),
        (unchanged, unchanged_label),
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
    use djangofmt_formatter::line_width::{IndentWidth, LineLength, SelfClosing};
    use djangofmt_formatter::panic::catch_unwind;
    use std::io;
    use std::path::PathBuf;

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
    fn format_command_error_write_display_stdin_path() {
        let io_err = io::Error::other("disk full");
        let err = CommandError::Write(None, io_err);
        assert_eq!(err.to_string(), "Failed to write -: disk full");
    }

    #[test]
    fn format_command_error_panic_display() {
        let panic_error = catch_unwind(|| panic!("boom")).unwrap_err();
        let err = CommandError::Panic(Some(PathBuf::from("a.html")), Box::new(panic_error));

        let full = err.to_string();
        assert!(full.contains("Panicked while processing a.html"), "{full}");
        assert!(full.contains("boom"), "{full}");
        assert!(full.contains("issues/new"), "{full}");
        assert!(err.concise().contains("a.html: Panicked: boom"));
    }

    #[test]
    fn merge_custom_blocks_both_none() {
        assert_eq!(merge_custom_blocks(None, None), None);
    }

    #[rstest]
    #[case::only_cli(Some(vec!["foo", "bar"]), None, vec!["bar", "foo"])]
    #[case::only_pyproject(None, Some(vec!["baz"]), vec!["baz"])]
    #[case::both_without_overlap(Some(vec!["foo"]), Some(vec!["bar"]), vec!["bar", "foo"])]
    #[case::both_with_duplicates(
        Some(vec!["foo", "bar"]),
        Some(vec!["bar", "baz"]),
        vec!["bar", "baz", "foo"],
    )]
    fn merge_custom_blocks_cases(
        #[case] cli: Option<Vec<&str>>,
        #[case] pyproject: Option<Vec<&str>>,
        #[case] expected: Vec<&str>,
    ) {
        let cli = cli.map(to_strings);
        let pyproject = pyproject.map(to_strings);
        let mut result = merge_custom_blocks(cli, pyproject).unwrap();

        result.sort();
        assert_eq!(result, to_strings(expected));
    }

    fn to_strings(values: Vec<&str>) -> Vec<String> {
        values.into_iter().map(String::from).collect()
    }

    #[test]
    fn formatter_config_from_args_defaults() {
        let args = FormatCommand::default();
        let pyproject = PyprojectSettings::default();
        let config =
            formatter_config_from_args(&args, &pyproject, &EditorconfigSettings::default());
        assert_eq!(config.markup.layout.print_width, 120);
        assert_eq!(config.markup.layout.indent_width, 4);
    }

    #[test]
    fn formatter_config_from_args_cli_overrides_pyproject() {
        let args = FormatCommand {
            line_length: Some(LineLength::try_from(80u16).unwrap()),
            indent_width: Some(IndentWidth::try_from(2u8).unwrap()),
            html_void_self_closing: Some(SelfClosing::Always),
            ..Default::default()
        };
        let pyproject = PyprojectSettings {
            line_length: Some(LineLength::try_from(200u16).unwrap()),
            indent_width: Some(IndentWidth::try_from(8u8).unwrap()),
            html_void_self_closing: Some(SelfClosing::Never),
            ..Default::default()
        };
        let config =
            formatter_config_from_args(&args, &pyproject, &EditorconfigSettings::default());
        assert_eq!(config.markup.layout.print_width, 80);
        assert_eq!(config.markup.layout.indent_width, 2);
        assert_eq!(config.markup.language.html_void_self_closing, Some(true));
    }

    #[test]
    fn formatter_config_from_args_falls_back_to_pyproject() {
        let args = FormatCommand::default();
        let pyproject = PyprojectSettings {
            line_length: Some(LineLength::try_from(200u16).unwrap()),
            ..Default::default()
        };
        let config =
            formatter_config_from_args(&args, &pyproject, &EditorconfigSettings::default());
        assert_eq!(config.markup.layout.print_width, 200);
    }

    #[test]
    fn formatter_config_from_args_falls_back_to_editorconfig() {
        let args = FormatCommand::default();
        let editorconfig = EditorconfigSettings {
            line_length: Some(LineLength::try_from(100u16).unwrap()),
            indent_width: Some(IndentWidth::try_from(2u8).unwrap()),
        };
        let config =
            formatter_config_from_args(&args, &PyprojectSettings::default(), &editorconfig);
        assert_eq!(config.markup.layout.print_width, 100);
        assert_eq!(config.markup.layout.indent_width, 2);
    }

    #[test]
    fn formatter_config_preserve_unquoted_attrs_from_cli() {
        let args = FormatCommand {
            preserve_unquoted_attrs: true,
            ..Default::default()
        };
        let pyproject = PyprojectSettings::default();
        let config =
            formatter_config_from_args(&args, &pyproject, &EditorconfigSettings::default());
        assert!(config.markup.language.preserve_unquoted_attrs);
    }

    #[test]
    fn formatter_config_preserve_unquoted_attrs_from_pyproject() {
        let args = FormatCommand::default();
        let pyproject = PyprojectSettings {
            preserve_unquoted_attrs: Some(true),
            ..Default::default()
        };
        let config =
            formatter_config_from_args(&args, &pyproject, &EditorconfigSettings::default());
        assert!(config.markup.language.preserve_unquoted_attrs);
    }

    #[test]
    fn formatter_config_no_preserve_unquoted_attrs_cli_overrides_pyproject() {
        let args = FormatCommand {
            no_preserve_unquoted_attrs: true,
            ..Default::default()
        };
        let pyproject = PyprojectSettings {
            preserve_unquoted_attrs: Some(true),
            ..Default::default()
        };
        let config =
            formatter_config_from_args(&args, &pyproject, &EditorconfigSettings::default());
        assert!(!config.markup.language.preserve_unquoted_attrs);
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
        assert_eq!(build_summary(&results, false), expected);
    }

    #[rstest]
    #[case(vec![FormatResult::Formatted], "1 file would be reformatted")]
    #[case(vec![
        FormatResult::Formatted,
        FormatResult::Unchanged,
        FormatResult::Skipped,
    ], "1 file would be reformatted, 1 file already formatted, 1 file skipped")]
    fn test_write_summary_check_mode(#[case] results: Vec<FormatResult>, #[case] expected: &str) {
        assert_eq!(build_summary(&results, true), expected);
    }
}

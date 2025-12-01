use markup_fmt::{FormatError, Language, SyntaxErrorKind, format_text};
use miette::{Diagnostic, NamedSource, SourceSpan};
use rayon::iter::Either::{Left, Right};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, error};

use crate::ExitStatus;
use crate::args::{FormatCommand, GlobalConfigArgs, Profile};
use crate::error::Result;
use crate::logging::LogLevel;

/// Pre-built configuration for all formatters.
pub struct FormatterConfig {
    /// Config for main HTML/Jinja formatter
    pub markup: markup_fmt::config::FormatOptions,
    /// Config for CSS/SCSS formatter
    pub malva: malva::config::FormatOptions,
}

impl FormatterConfig {
    #[must_use]
    pub fn new(
        print_width: usize,
        indent_width: usize,
        custom_blocks: Option<Vec<String>>,
    ) -> Self {
        Self {
            markup: build_markup_options(print_width, indent_width, custom_blocks),
            malva: build_malva_config(print_width, indent_width),
        }
    }
}

const DJANGOFMT_IGNORE_COMMENT: &str = "djangofmt:ignore";

/// Build default `markup_fmt` options for HTML/Jinja formatting.
#[must_use]
pub fn build_markup_options(
    print_width: usize,
    indent_width: usize,
    custom_blocks: Option<Vec<String>>,
) -> markup_fmt::config::FormatOptions {
    markup_fmt::config::FormatOptions {
        layout: markup_fmt::config::LayoutOptions {
            print_width,
            indent_width,
            ..markup_fmt::config::LayoutOptions::default()
        },
        language: markup_fmt::config::LanguageOptions {
            format_comments: false,
            // See https://developer.mozilla.org/en-US/docs/Glossary/Void_element#self-closing_tags
            //  `<br/>` -> `<br>`
            html_void_self_closing: Some(false),
            // `<circle cx="50" cy="50" r="50">` -> ParseError
            // `<circle cx="50" cy="50" r="50"></circle>` -> `<circle cx="50" cy="50" r="50" />`
            svg_self_closing: Some(true),
            // Same reasoning as SVG
            mathml_self_closing: Some(true),
            // `<div/>desfsdf` -> `<div></div>desfsdf`
            // This is actually still incorrect (but slightly better than nothing), we need `<div>desfsdf</div>` (or a parse error)
            html_normal_self_closing: Some(false),
            // This is actually nice to keep this setting false, it makes it possible to control wrapping
            // of props semi manually by inserting or not a newline before the first prop.
            // See https://github.com/g-plane/markup_fmt/issues/10 that showcase this.
            prefer_attrs_single_line: false,
            // Parse some additional custom blocks, for ex "stage,cache,flatblock,section,csp_compress"
            custom_blocks,
            // Custom ignore comment directives for djangofmt
            ignore_comment_directive: DJANGOFMT_IGNORE_COMMENT.into(),
            ignore_file_comment_directive: DJANGOFMT_IGNORE_COMMENT.into(),
            style_indent: true,
            ..markup_fmt::config::LanguageOptions::default()
        },
    }
}

/// Build default `malva` options for CSS/SCSS/SASS/LESS formatting.
fn build_malva_config(print_width: usize, indent_width: usize) -> malva::config::FormatOptions {
    malva::config::FormatOptions {
        layout: malva::config::LayoutOptions {
            print_width,
            indent_width,
            ..malva::config::LayoutOptions::default()
        },
        language: malva::config::LanguageOptions {
            // Because markup_fmt uses DoubleQuotes
            quotes: malva::config::Quotes::AlwaysSingle,
            operator_linebreak: malva::config::OperatorLineBreak::Before,
            format_comments: true,
            linebreak_in_pseudo_parens: true,
            // Todo: "smacss" or "concentric" seem nice
            declaration_order: None,
            // TODO: "keyword" or "percentage" would be nice for consistency
            keyframe_selector_notation: None,
            // TODO: We might want to switch that to false if we properly handle multiline style attr.
            single_line_top_level_declarations: true,
            selector_override_comment_directive: "djangofmt-selector-override".into(),
            ignore_comment_directive: DJANGOFMT_IGNORE_COMMENT.into(),
            ignore_file_comment_directive: DJANGOFMT_IGNORE_COMMENT.into(),
            ..malva::config::LanguageOptions::default()
        },
    }
}

pub fn format(args: FormatCommand, global_options: &GlobalConfigArgs) -> Result<ExitStatus> {
    let config = FormatterConfig::new(args.line_length, args.indent_width, args.custom_blocks);

    let start = Instant::now();
    let (results, mut errors): (Vec<_>, Vec<_>) = args
        .files
        .par_iter()
        .map(|entry| {
            let path = entry.as_path();
            format_path(path, &config, &args.profile)
        })
        .partition_map(|result| match result {
            Ok(diagnostic) => Left(diagnostic),
            Err(err) => Right(err),
        });

    let duration = start.elapsed();
    debug!(
        "Formatted {} files in {:.2?}",
        results.len() + errors.len(),
        duration
    );

    // Report on any errors.
    errors.sort_unstable_by(|a, b| a.path().cmp(&b.path()));
    let error_count = errors.len();
    for error in errors {
        eprintln!("{:?}", miette::Report::new(*error));
    }
    if error_count > 0 {
        error!("Couldn't format {} files!", error_count);
    }

    // Report on the formatting changes.
    if global_options.log_level() >= LogLevel::Default {
        write_summary(results.as_ref())?;
    }

    if error_count == 0 {
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::Failure)
    }
}

/// Format the file at the given [`Path`].
#[tracing::instrument(level="debug", skip_all, fields(path = %path.display()))]
fn format_path(
    path: &Path,
    config: &FormatterConfig,
    profile: &Profile,
) -> std::result::Result<FormatResult, Box<FormatCommandError>> {
    // Extract the source from the file.
    let unformatted = std::fs::read_to_string(path)
        .map_err(|err| FormatCommandError::Read(Some(path.to_path_buf()), err))?;

    // Format the source.
    let format_result = format_text(
        &unformatted,
        Language::from(profile),
        &config.markup,
        |code, hints| -> Result<Cow<str>> {
            match hints.ext {
                "css" | "scss" | "sass" | "less" => {
                    let mut malva_config = config.malva.clone();
                    malva_config.layout.print_width = hints.print_width;
                    Ok(malva::format_text(
                        code,
                        malva::detect_syntax(path).unwrap_or(malva::Syntax::Css),
                        &malva_config,
                    )
                    // TODO: Don't skip errors and actually handle these cases.
                    //       Currently we have errors when there is templating blocks inside style tags
                    // .map_err(anyhow::Error::from)
                    .map_or_else(|_| code.into(), Cow::from))
                }
                _ => Ok(code.into()),
            }
        },
    );

    let formatted = format_result.map_err(|err| {
        FormatCommandError::Parse(ParseError::new(
            Some(path.to_path_buf()),
            unformatted.clone(),
            &err,
        ))
    })?;

    // Checked if something changed and write to file if necessary
    if formatted.len() == unformatted.len() && formatted == unformatted {
        Ok(FormatResult::Unchanged)
    } else {
        let mut writer = File::create(path)
            .map_err(|err| FormatCommandError::Write(Some(path.to_path_buf()), err))?;

        writer
            .write_all(formatted.as_bytes())
            .map_err(|err| FormatCommandError::Write(Some(path.to_path_buf()), err))?;

        Ok(FormatResult::Formatted)
    }
}

/// An error that can occur while formatting a set of files.
#[derive(Debug, thiserror::Error, Diagnostic)]
pub enum FormatCommandError {
    #[error("Failed to read {path}: {err}", path = path_display(.0.as_ref()), err = .1)]
    Read(Option<PathBuf>, io::Error),
    #[error("{}", .0.message)]
    #[diagnostic(transparent)]
    Parse(ParseError),
    #[error("Failed to write {path}: {err}", path = path_display(.0.as_ref()), err = .1)]
    Write(Option<PathBuf>, io::Error),
}

fn path_display(path: Option<&PathBuf>) -> String {
    path.map_or_else(|| "<unknown>".to_string(), |p| p.display().to_string())
}

impl FormatCommandError {
    fn path(&self) -> Option<&Path> {
        match self {
            Self::Parse(err) => err.path.as_deref(),
            Self::Read(path, _) | Self::Write(path, _) => path.as_deref(),
        }
    }
}

#[derive(Debug, Diagnostic, thiserror::Error)]
#[error("{message}")]
pub struct ParseError {
    path: Option<PathBuf>,
    message: String,
    #[source_code]
    src: NamedSource<String>,
    #[label("here")]
    span: SourceSpan,
}

impl ParseError {
    #[must_use]
    pub fn new<E: std::fmt::Debug>(
        path: Option<PathBuf>,
        source: String,
        err: &FormatError<E>,
    ) -> Self {
        let (message, offset) = match err {
            FormatError::Syntax(syntax_err) => {
                match &syntax_err.kind {
                    // Point to the opening tag instead of where the error was detected (which is always the end of the file)
                    SyntaxErrorKind::ExpectCloseTag {
                        tag_name,
                        line,
                        column,
                    } => (
                        format!("expected close tag for opening tag <{tag_name}>",),
                        line_col_to_offset(&source, *line, *column),
                    ),
                    _ => (syntax_err.kind.to_string(), syntax_err.pos),
                }
            }
            FormatError::External(errors) => {
                let msg = errors
                    .iter()
                    .map(|e| format!("{e:?}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                (format!("external formatter error: {msg}"), 0)
            }
        };
        let name = path
            .as_ref()
            .map_or_else(|| "<unknown>".to_string(), |p| p.display().to_string());
        Self {
            path,
            message,
            src: NamedSource::new(name, source),
            span: SourceSpan::from(offset),
        }
    }
}

/// Convert 1-indexed line and column to a byte offset in the source.
fn line_col_to_offset(source: &str, line: usize, column: usize) -> usize {
    let mut offset = 0;
    for (i, src_line) in source.lines().enumerate() {
        if i + 1 == line {
            // Found the line, add column offset (1-indexed)
            return offset + column.saturating_sub(1);
        }
        // +1 for the newline character
        offset += src_line.len() + 1;
    }
    // Fallback to end of file
    source.len()
}

/// The result of an individual formatting operation.
#[derive(Eq, PartialEq, Hash, Debug)]
enum FormatResult {
    /// The file was formatted.
    Formatted,

    /// The file was unchanged, as the formatted contents matched the existing contents.
    Unchanged,
}

/// Write a summary of the formatting results to stdout.
fn write_summary(results: &[FormatResult]) -> Result<()> {
    let mut counts = HashMap::new();
    for val in results {
        counts
            .entry(val)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }
    let stdout = &mut io::stdout().lock();

    let changed = counts.get(&FormatResult::Formatted).copied().unwrap_or(0);
    let unchanged = counts.get(&FormatResult::Unchanged).copied().unwrap_or(0);
    if changed > 0 && unchanged > 0 {
        writeln!(
            stdout,
            "{} file{} reformatted, {} file{} left unchanged !",
            changed,
            if changed == 1 { "" } else { "s" },
            unchanged,
            if unchanged == 1 { "" } else { "s" },
        )?;
    } else if changed > 0 {
        writeln!(
            stdout,
            "{} file{} reformatted !",
            changed,
            if changed == 1 { "" } else { "s" },
        )?;
    } else if unchanged > 0 {
        writeln!(
            stdout,
            "{} file{} left unchanged !",
            unchanged,
            if unchanged == 1 { "" } else { "s" },
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_command_error_read_display() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err = FormatCommandError::Read(Some(PathBuf::from("/path/to/file.html")), io_err);
        assert_eq!(
            err.to_string(),
            "Failed to read /path/to/file.html: file not found"
        );
    }

    #[test]
    fn format_command_error_read_display_unknown_path() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        let err = FormatCommandError::Read(None, io_err);
        assert_eq!(
            err.to_string(),
            "Failed to read <unknown>: permission denied"
        );
    }

    #[test]
    fn format_command_error_write_display() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        let err = FormatCommandError::Write(Some(PathBuf::from("/path/to/output.html")), io_err);
        assert_eq!(
            err.to_string(),
            "Failed to write /path/to/output.html: permission denied"
        );
    }

    #[test]
    fn format_command_error_write_display_unknown_path() {
        let io_err = io::Error::other("disk full");
        let err = FormatCommandError::Write(None, io_err);
        assert_eq!(err.to_string(), "Failed to write <unknown>: disk full");
    }
}

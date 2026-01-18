use miette::{Diagnostic, NamedSource, SourceOffset, SourceSpan};
use rayon::iter::Either::{Left, Right};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, error, info};

use crate::ExitStatus;
use crate::args::{FormatCommand, Profile};
use crate::error::Result;

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

const DJANGOFMT_IGNORE_COMMENT_DIRECTIVE: &str = "djangofmt:ignore";
const DJANGOFMT_IGNORE_COMMENT: &str = "<!-- djangofmt:ignore -->";

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
            // HTML void elements should not be self-closing:
            // See https://developer.mozilla.org/en-US/docs/Glossary/Void_element#self-closing_tags
            // <br/> -> <br>
            html_void_self_closing: Some(false),
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

pub fn format(args: FormatCommand) -> Result<ExitStatus> {
    let config = FormatterConfig::new(
        args.options.line_length,
        args.options.indent_width,
        args.options.custom_blocks,
    );

    let start = Instant::now();
    let (results, mut parse_errors): (Vec<_>, Vec<_>) = args
        .files
        .par_iter()
        .map(|entry| format_path(entry, &config, &args.options.profile))
        .partition_map(|result| match result {
            Ok(fmt_res) => Left(fmt_res),
            Err(err) => Right(err),
        });

    let duration = start.elapsed();
    debug!("Formatted {} files in {:.2?}", args.files.len(), duration);

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
    write_summary(results.as_ref());

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
    profile: &Profile,
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
    profile: &Profile,
) -> std::result::Result<FormatResult, Box<FormatCommandError>> {
    let unformatted = std::fs::read_to_string(path)
        .map_err(|err| FormatCommandError::Read(Some(path.to_path_buf()), err))?;

    let formatted = format_text(&unformatted, config, profile).map_err(|err| {
        FormatCommandError::Parse(ParseError::new(
            Some(path.to_path_buf()),
            unformatted.clone(),
            &err,
        ))
    })?;

    let Some(formatted) = formatted else {
        return Ok(FormatResult::Skipped);
    };

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

#[must_use]
pub fn path_display(path: Option<&PathBuf>) -> String {
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
    pub path: Option<PathBuf>,
    pub message: String,
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
        err: &markup_fmt::FormatError<E>,
    ) -> Self {
        let (message, offset) = match err {
            markup_fmt::FormatError::Syntax(syntax_err) => {
                match &syntax_err.kind {
                    // Point to the opening tag instead of where the error was detected (which is always the end of the file)
                    markup_fmt::SyntaxErrorKind::ExpectCloseTag {
                        tag_name,
                        line,
                        column,
                    } => (
                        format!("expected close tag for opening tag <{tag_name}>",),
                        SourceOffset::from_location(&source, *line, *column),
                    ),
                    _ => (syntax_err.kind.to_string(), syntax_err.pos.into()),
                }
            }
            markup_fmt::FormatError::External(errors) => {
                let msg = errors
                    .iter()
                    .map(|e| format!("{e:?}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                (format!("external formatter error: {msg}"), 0.into())
            }
        };
        let name = path
            .as_ref()
            .map_or_else(|| "<unknown>".to_string(), |p| p.display().to_string());
        Self {
            path,
            message,
            src: NamedSource::new(name, source),
            span: offset.into(),
        }
    }
}

/// The result of an individual formatting operation.
#[derive(Eq, PartialEq, Hash, Debug)]
enum FormatResult {
    /// The file was formatted.
    Formatted,

    /// The file was unchanged, as the formatted contents matched the existing contents.
    Unchanged,

    /// The file was skipped due to a top-level ignore comment.
    Skipped,
}

/// Write a summary of the formatting results to stdout.
fn write_summary(results: &[FormatResult]) {
    let mut counts = HashMap::new();
    for val in results {
        *counts.entry(val).or_insert(0) += 1;
    }

    let changed = counts.get(&FormatResult::Formatted).copied().unwrap_or(0);
    let unchanged = counts.get(&FormatResult::Unchanged).copied().unwrap_or(0);
    let skipped = counts.get(&FormatResult::Skipped).copied().unwrap_or(0);

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

    if !parts.is_empty() {
        info!("{} !", parts.join(", "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;
    use serial_test::serial;
    use tracing_test::traced_test;
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
    #[traced_test]
    #[rstest]
    #[case(vec![], "")]
    #[case(vec![FormatResult::Formatted], "1 file reformatted !")]
    #[case(vec![FormatResult::Formatted, FormatResult::Formatted], "2 files reformatted !")]
    #[case(vec![FormatResult::Unchanged], "1 file left unchanged !")]
    #[case(vec![FormatResult::Unchanged, FormatResult::Unchanged], "2 files left unchanged !")]
    #[case(vec![FormatResult::Skipped], "1 file skipped !")]
    #[case(vec![FormatResult::Skipped, FormatResult::Skipped], "2 files skipped !")]
    #[case(vec![FormatResult::Formatted, FormatResult::Unchanged], "1 file reformatted, 1 file left unchanged !")]
    #[case(vec![FormatResult::Formatted, FormatResult::Formatted, FormatResult::Unchanged], "2 files reformatted, 1 file left unchanged !")]
    #[case(vec![FormatResult::Formatted, FormatResult::Skipped], "1 file reformatted, 1 file skipped !")]
    #[case(vec![FormatResult::Formatted, FormatResult::Skipped, FormatResult::Skipped], "1 file reformatted, 2 files skipped !")]
    #[case(vec![FormatResult::Unchanged, FormatResult::Skipped], "1 file left unchanged, 1 file skipped !")]
    #[case(vec![FormatResult::Unchanged, FormatResult::Unchanged, FormatResult::Skipped], "2 files left unchanged, 1 file skipped !")]
    #[case(vec![FormatResult::Formatted, FormatResult::Unchanged, FormatResult::Skipped], "1 file reformatted, 1 file left unchanged, 1 file skipped !")]
    #[case(vec![
        FormatResult::Formatted,
        FormatResult::Formatted,
        FormatResult::Unchanged,
        FormatResult::Skipped,
        FormatResult::Skipped,
        FormatResult::Skipped,
    ], "2 files reformatted, 1 file left unchanged, 3 files skipped !")]
    #[serial]
    fn test_write_summary(#[case] results: Vec<FormatResult>, #[case] expected: &str) {
        write_summary(&results);

        if expected.is_empty() {
            logs_assert(|lines: &[&str]| {
                if lines.is_empty() {
                    Ok(())
                } else {
                    Err("Expected no logs, but found some".into())
                }
            });
        } else {
            assert!(logs_contain(expected));
        }
    }
}

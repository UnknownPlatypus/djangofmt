use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use markup_fmt::config::{FormatOptions, LanguageOptions, LayoutOptions};
use markup_fmt::{FormatError, Language, format_text};
use pretty_jinja;
use pretty_jinja::config::OperatorLineBreak;
use rayon::iter::Either::{Left, Right};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tracing::{debug, error};

use crate::ExitStatus;
use crate::args::{FormatCommand, GlobalConfigArgs, Profile};
use crate::error::{Error, Result};
use crate::logging::LogLevel;

pub fn format(args: FormatCommand, global_options: &GlobalConfigArgs) -> Result<ExitStatus> {
    let format_options = FormatOptions {
        layout: LayoutOptions {
            print_width: args.line_length,
            indent_width: args.indent_width,
            ..LayoutOptions::default()
        },
        language: LanguageOptions {
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
            custom_blocks: args.custom_blocks,
            // Custom ignore comment directives for djangofmt
            ignore_comment_directive: "djangofmt:ignore".into(),
            ignore_file_comment_directive: "djangofmt:ignore".into(),
            ..LanguageOptions::default()
        },
    };

    let start = Instant::now();
    let (results, mut errors): (Vec<_>, Vec<_>) = args
        .files
        .par_iter()
        .map(|entry| {
            let path = entry.as_path();
            // Format the source.
            format_path(path, &format_options, &args.profile)
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
    for error in &errors {
        error!("{error}");
    }
    if !errors.is_empty() {
        error!("Couldn't format {} files!", errors.len());
    }

    // Report on the formatting changes.
    if global_options.log_level() >= LogLevel::Default {
        write_summary(results.as_ref())?;
    }

    if errors.is_empty() {
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::Failure)
    }
}

/// Format the file at the given [`Path`].
#[tracing::instrument(level="debug", skip_all, fields(path = %path.display()))]
fn format_path(
    path: &Path,
    format_options: &FormatOptions,
    profile: &Profile,
) -> std::result::Result<FormatResult, FormatCommandError> {
    // Extract the source from the file.
    let unformatted = std::fs::read_to_string(path)
        .map_err(|err| FormatCommandError::Read(Some(path.to_path_buf()), err))?;

    // Format the source.
    let format_result = format_text(
        &unformatted,
        Language::from(profile),
        format_options,
        |code, hints| -> Result<Cow<str>> {
            let ext = hints.ext;
            let additional_config =
                dprint_plugin_markup::build_additional_config(hints, format_options);
            if let Some(syntax) = malva::detect_syntax(Path::new("file").with_extension(ext)) {
                Ok(malva::format_text(
                    code,
                    syntax,
                    &serde_json::to_value(additional_config).and_then(serde_json::from_value)?,
                )
                // TODO: Don't skip errors and actually handle these cases.
                //       Currently we have errors when there is templating blocks inside style tags
                // .map_err(anyhow::Error::from)
                .map_or_else(|_| code.into(), Cow::from))
            } else {
                let pretty_jinja_config = pretty_jinja::config::FormatOptions {
                    layout: serde_json::from_value(serde_json::to_value(&format_options.layout)?)?,
                    language: pretty_jinja::config::LanguageOptions {
                        operator_linebreak: OperatorLineBreak::Before,
                        trailing_comma: pretty_jinja::config::TrailingComma::OnlyMultiLine,
                        args_trailing_comma: None,
                        expr_dict_trailing_comma: None,
                        expr_list_trailing_comma: None,
                        expr_tuple_trailing_comma: None,
                        params_trailing_comma: None,
                        prefer_single_line: true,
                        args_prefer_single_line: Some(true),
                        expr_dict_prefer_single_line: Some(true),
                        expr_list_prefer_single_line: Some(true),
                        expr_tuple_prefer_single_line: Some(true),
                        params_prefer_single_line: Some(true),
                        brace_spacing: true,
                        bracket_spacing: false,
                        args_paren_spacing: false,
                        params_paren_spacing: false,
                        tuple_paren_spacing: false,
                    },
                };
                Ok(match ext {
                    "markup-fmt-jinja-expr" => {
                        pretty_jinja::format_expr(code, &pretty_jinja_config)
                            .map_or_else(|_| code.into(), Cow::from)
                    }
                    "markup-fmt-jinja-stmt" => {
                        pretty_jinja::format_stmt(code, &pretty_jinja_config)
                            .map_or_else(|_| code.into(), Cow::from)
                    }
                    _ => code.into(),
                })

                // dprint_plugin_biome::format_text(
                //     &Path::new("file").with_extension(ext),
                //     code,
                //     &serde_json::to_value(additional_config)
                //         .and_then(serde_json::from_value)
                //         .unwrap_or_default(),
                // )
                //     .map(|formatted| {
                //         if let Some(formatted) = formatted {
                //             Cow::from(formatted)
                //         } else {
                //             Cow::from(code)
                //         }
                //     })
            }
        },
    );

    let formatted =
        format_result.map_err(|err| FormatCommandError::Parse(Some(path.to_path_buf()), err))?;

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
#[derive(Debug)]
pub enum FormatCommandError {
    Read(Option<PathBuf>, io::Error),
    Parse(Option<PathBuf>, FormatError<Error>),
    Write(Option<PathBuf>, io::Error),
}

impl FormatCommandError {
    fn path(&self) -> Option<&Path> {
        match self {
            Self::Parse(path, _) | Self::Read(path, _) | Self::Write(path, _) => path.as_deref(),
        }
    }
}

impl Display for FormatCommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(path, err) => {
                if let Some(path) = path {
                    write!(f, "Failed to parse {} with error {err:?}", path.display())
                } else {
                    write!(f, "Failed to parse with error {err:?}")
                }
            }
            Self::Read(path, err) => {
                if let Some(path) = path {
                    write!(f, "Failed to read {} with error {err:?}", path.display())
                } else {
                    write!(f, "Failed to read with error {err:?}",)
                }
            }
            Self::Write(path, err) => {
                if let Some(path) = path {
                    write!(f, "Failed to write {} with error {err:?}", path.display())
                } else {
                    write!(f, "Failed to write with error {err:?}")
                }
            }
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

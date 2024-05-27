use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;
use markup_fmt::{format_text, FormatError, Language};
use markup_fmt::config::{FormatOptions, LanguageOptions, LayoutOptions};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rayon::iter::Either::{Left, Right};
use tracing::{debug, error};

use crate::args::{FormatCommand, GlobalConfigArgs};
use crate::ExitStatus;
use crate::logging::LogLevel;

pub(crate) fn format(args: FormatCommand, global_options: GlobalConfigArgs) -> Result<ExitStatus> {
    let format_options = FormatOptions {
        layout: LayoutOptions {
            print_width: args.line_length.unwrap_or(120),
            indent_width: 4,
            ..LayoutOptions::default()
        },
        language: LanguageOptions {
            closing_bracket_same_line: false, // This is default, remove later
            ..LanguageOptions::default()
        },
    };

    // TODO handle cache here

    let start = Instant::now();
    let (results, mut errors): (Vec<_>, Vec<_>) = args
        .files
        .par_iter()
        .map(|entry| {
            let path = entry.as_path();
            // println!("Formatting {}", path.display());

            // Format the source.
            format_path(path, &format_options)
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

    // Report on the formatting changes.
    if global_options.log_level() >= LogLevel::Default {
        write_summary(results)?;
    }
    if errors.is_empty() {
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::Error)
    }
}

/// Write a summary of the formatting results to stdout.
fn write_summary(results: Vec<FormatResult>) -> Result<()> {
    let mut counts = HashMap::new();
    results.iter().for_each(|val| {
        counts
            .entry(val)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    });
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

/// Format the file at the given [`Path`].
#[tracing::instrument(level="debug", skip_all, fields(path = %path.display()))]
pub(crate) fn format_path(
    path: &Path,
    format_options: &FormatOptions,
) -> Result<FormatResult, FormatCommandError> {
    // Extract the source from the file.
    let unformatted = match std::fs::read_to_string(path) {
        Ok(unformatted) => unformatted,
        Err(err) => return Err(FormatCommandError::Read(Some(path.to_path_buf()), err)),
    };

    // Format the source.
    let formatted = match format_text(
        &unformatted,
        Language::Jinja,
        format_options,
        |_, code, _| Ok::<_, ()>(code.into()),
    ) {
        Ok(formatted) => formatted,
        Err(err) => return Err(FormatCommandError::Parse(Some(path.to_path_buf()), err)),
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
#[derive(Debug)]
pub(crate) enum FormatCommandError {
    Read(Option<PathBuf>, io::Error),
    Parse(Option<PathBuf>, FormatError<()>),
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
                    write!(f, "Failed to parse {:?} with error {err:?}", path)
                } else {
                    write!(f, "Failed to parse with error {err:?}")
                }
            }
            Self::Read(path, err) => {
                if let Some(path) = path {
                    write!(f, "Failed to read {path:?} with error {err:?}",)
                } else {
                    write!(f, "Failed to read with error {err:?}",)
                }
            }
            Self::Write(path, err) => {
                if let Some(path) = path {
                    write!(f, "Failed to write {path:?} with error {err:?}")
                } else {
                    write!(f, "Failed to write with error {err:?}")
                }
            }
        }
    }
}
/// The result of an individual formatting operation.
#[derive(Eq, PartialEq, Hash, Debug)]
pub(crate) enum FormatResult {
    /// The file was formatted.
    Formatted,

    /// The file was unchanged, as the formatted contents matched the existing contents.
    Unchanged,
}
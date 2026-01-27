use crate::commands::format::path_display;
use djangofmt_lint::{FileDiagnostics, Settings, check_ast};
use markup_fmt::FormatError;
use markup_fmt::parser::Parser;
use miette::{Diagnostic, NamedSource};
use rayon::iter::Either::{Left, Right};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::ExitStatus;
use crate::args::Profile;
use crate::commands::format::ParseError;
use crate::error::Result;
use std::time::Instant;
use tracing::{debug, error};

/// Check the given source code for linting errors.
pub fn check(args: &CheckCommand) -> Result<ExitStatus> {
    let start = Instant::now();
    let (file_diagnostics, mut parse_errors): (Vec<_>, Vec<_>) = args
        .files
        .par_iter()
        .map(|path| check_path(path, &args.profile))
        .partition_map(|result| match result {
            Ok(diags) => Left(diags),
            Err(err) => Right(err),
        });

    let duration = start.elapsed();
    debug!("Checked {} files in {:.2?}", args.files.len(), duration);

    // Report on any parsing errors.
    parse_errors.sort_unstable_by(|a, b| a.path().cmp(&b.path()));
    let error_count = parse_errors.len();
    for error in parse_errors {
        error!("{:?}", miette::Report::new(*error));
    }
    if error_count > 0 {
        error!("Couldn't check {} files!", error_count);
    }

    // Filter out files with no diagnostics and count total
    let file_diagnostics: Vec<_> = file_diagnostics
        .into_iter()
        .filter(|fd| !fd.is_empty())
        .collect();
    let total_diagnostics: usize = file_diagnostics.iter().map(FileDiagnostics::len).sum();

    if total_diagnostics == 0 && error_count == 0 {
        return Ok(ExitStatus::Success);
    }

    // Report diagnostics per file
    for file_diag in file_diagnostics {
        error!("{:?}", miette::Report::new(file_diag));
    }

    Ok(ExitStatus::Failure)
}

/// Check the file at the given [`Path`] for linting issues.
#[tracing::instrument(level = "debug", skip_all, fields(path = %path.display()))]
fn check_path(
    path: &Path,
    profile: &Profile,
) -> std::result::Result<FileDiagnostics, Box<CheckCommandError>> {
    let source = fs::read_to_string(path)
        .map_err(|err| CheckCommandError::Read(Some(path.to_path_buf()), err))?;

    let mut parser = Parser::new(&source, profile.into(), vec![]);
    let ast = parser.parse_root().map_err(|err| {
        CheckCommandError::Parse(ParseError::new(
            Some(path.to_path_buf()),
            source.clone(),
            &FormatError::<markup_fmt::SyntaxError>::Syntax(err),
        ))
    })?;

    let settings = Settings::default();
    let diagnostics = check_ast(&ast, &settings);

    if diagnostics.is_empty() {
        return Ok(FileDiagnostics::empty());
    }
    Ok(FileDiagnostics::new(
        NamedSource::new(path.to_string_lossy(), source),
        diagnostics,
    ))
}

/// An error that can occur while linting a set of files.
#[derive(Debug, thiserror::Error, Diagnostic)]
pub enum CheckCommandError {
    #[error("Failed to read {path}: {err}", path = path_display(.0.as_ref()), err = .1)]
    Read(Option<PathBuf>, io::Error),
    #[error("{}", .0.message)]
    #[diagnostic(transparent)]
    Parse(ParseError),
}
impl CheckCommandError {
    fn path(&self) -> Option<&Path> {
        match self {
            Self::Parse(err) => err.path.as_deref(),
            Self::Read(path, _) => path.as_deref(),
        }
    }
}
#[derive(Clone, Debug, clap::Parser, Deserialize)]
pub struct CheckCommand {
    /// List of files to check.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
    /// Template language profile to use
    #[arg(long, value_enum, default_value = "django")]
    pub profile: Profile,
}

use djangofmt_lint::{FileDiagnostics, Settings, check_ast};
use markup_fmt::FormatError;
use markup_fmt::parser::Parser;
use miette::NamedSource;
use rayon::iter::Either::{Left, Right};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::fs;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error};

use crate::ExitStatus;
use crate::args::{CheckCommand, Profile};
use crate::error::{CommandError, ParseError, Result};

/// Check the given source code for linting errors.
pub fn check(args: &CheckCommand) -> Result<ExitStatus> {
    let resolved = super::resolve_command(&args.files, args.profile, &args.file_selection)?;

    let start = Instant::now();
    let (file_diagnostics, mut parse_errors): (Vec<_>, Vec<_>) = resolved
        .files
        .par_iter()
        .map(|path| check_path(path, resolved.profile))
        .partition_map(|result| match result {
            Ok(diags) => Left(diags),
            Err(err) => Right(err),
        });

    let duration = start.elapsed();
    debug!("Checked {} files in {:.2?}", resolved.files.len(), duration);

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
    profile: Option<Profile>,
) -> std::result::Result<FileDiagnostics, Box<CommandError>> {
    let profile = profile
        .or_else(|| Profile::from_path(path))
        .unwrap_or_default();
    let source = fs::read_to_string(path)
        .map_err(|err| CommandError::Read(Some(path.to_path_buf()), err))?;

    let mut parser = Parser::new(&source, profile.into(), vec![]);
    let ast = parser.parse_root().map_err(|err| {
        CommandError::Parse(ParseError::new(
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

use std::panic::UnwindSafe;
use std::path::{Path, PathBuf};

use tracing::error;

use crate::args::{FileSelectionArgs, OutputFormat};
use crate::error::{CommandError, Result};
use crate::pyproject::{PyprojectSettings, load_pyproject_from_cwd};
use crate::resolver::{ResolvedDiscoveryConfig, resolve_files};

pub mod check;
pub mod check_stdin;
pub mod format;
pub mod format_stdin;

/// Shared preamble for all commands: loads pyproject settings and discovers files.
pub(crate) struct ResolvedCommand {
    pub pyproject: PyprojectSettings,
    pub files: Vec<PathBuf>,
    /// Directory of the nearest `pyproject.toml` (or the cwd), anchoring path-relative config.
    pub project_root: PathBuf,
}

pub(crate) fn resolve_command(
    files: &[PathBuf],
    file_selection: &FileSelectionArgs,
) -> Result<ResolvedCommand> {
    let (pyproject, project_root) = load_pyproject_from_cwd()?;
    let discovery_config = ResolvedDiscoveryConfig::new(file_selection, &pyproject, &project_root);
    let resolved_files = resolve_files(files, &discovery_config)?;
    Ok(ResolvedCommand {
        pyproject,
        files: resolved_files,
        project_root,
    })
}

/// Run a per-file task, converting a panic into a [`CommandError::Panic`] so the run survives it.
///
/// `path` is [`None`] for a source with no backing file, such as stdin.
pub(crate) fn catch_file_panic<T>(
    path: Option<&Path>,
    f: impl FnOnce() -> std::result::Result<T, Box<CommandError>> + UnwindSafe,
) -> std::result::Result<T, Box<CommandError>> {
    djangofmt_formatter::panic::catch_unwind(f).unwrap_or_else(|error| {
        Err(Box::new(CommandError::Panic(
            path.map(Path::to_path_buf),
            Box::new(error),
        )))
    })
}

/// Log each error as a report, and return the count.
/// `verb` fills the summary line, e.g. "Couldn't format N files!".
///
/// Errors come in the resolved files' sorted order, which rayon keeps.
pub(crate) fn report_errors(
    errors: Vec<CommandError>,
    verb: &str,
    output_format: OutputFormat,
) -> usize {
    let count = errors.len();
    for err in errors {
        match output_format {
            OutputFormat::Full => error!("{:?}", miette::Report::new(err)),
            OutputFormat::Concise => error!("{}", err.concise()),
        }
    }
    if count > 0 {
        error!("Couldn't {verb} {count} files!");
    }
    count
}

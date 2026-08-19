use std::path::PathBuf;

use tracing::error;

use crate::args::{FileSelectionArgs, OutputFormat};
use crate::error::{CommandError, Result};
use crate::pyproject::{PyprojectSettings, load_pyproject_from_cwd};
use crate::resolver::{ResolvedDiscoveryConfig, resolve_files};

pub mod check;
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

/// Sort parse errors by path, log each as a report, and return the count.
/// `verb` fills the summary line, e.g. "Couldn't format N files!".
pub(crate) fn report_parse_errors(
    mut parse_errors: Vec<CommandError>,
    verb: &str,
    output_format: OutputFormat,
) -> usize {
    parse_errors.sort_unstable_by(|a, b| a.path().cmp(&b.path()));
    let count = parse_errors.len();
    for err in parse_errors {
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

use std::path::PathBuf;

use tracing::error;

use crate::args::{FileSelectionArgs, Profile};
use crate::error::{CommandError, Result};
use crate::pyproject::{PyprojectSettings, load_pyproject_from_cwd};
use crate::resolver::{ResolvedDiscoveryConfig, resolve_files};

pub mod check;
pub mod format;
pub mod format_stdin;

/// Shared preamble for all commands: loads pyproject settings, resolves profile, and discovers files.
pub(crate) struct ResolvedCommand {
    pub pyproject: PyprojectSettings,
    pub profile: Option<Profile>,
    pub files: Vec<PathBuf>,
}

pub(crate) fn resolve_command(
    files: &[PathBuf],
    profile: Option<Profile>,
    file_selection: &FileSelectionArgs,
) -> Result<ResolvedCommand> {
    let pyproject = load_pyproject_from_cwd()?;
    let profile = profile.or(pyproject.profile);
    let discovery_config = ResolvedDiscoveryConfig::new(file_selection, &pyproject);
    let resolved_files = resolve_files(files, &discovery_config)?;
    Ok(ResolvedCommand {
        pyproject,
        profile,
        files: resolved_files,
    })
}

/// Sort parse errors by path, log each as a report, and return the count.
/// `verb` fills the summary line, e.g. "Couldn't format N files!".
pub(crate) fn report_parse_errors(mut parse_errors: Vec<CommandError>, verb: &str) -> usize {
    parse_errors.sort_unstable_by(|a, b| a.path().cmp(&b.path()));
    let count = parse_errors.len();
    for err in parse_errors {
        error!("{:?}", miette::Report::new(err));
    }
    if count > 0 {
        error!("Couldn't {verb} {count} files!");
    }
    count
}

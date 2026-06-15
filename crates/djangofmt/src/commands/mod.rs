use std::path::{Path, PathBuf};

use crate::args::{FileSelectionArgs, Profile};
use crate::error::Result;
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
    /// Directory of the nearest `pyproject.toml` (or the cwd), used to anchor
    /// path-relative config such as `per-file-ignores`.
    pub project_root: PathBuf,
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
    let cwd = crate::fs::get_cwd();
    let project_root = crate::fs::find_nearest_ancestor_file(cwd, "pyproject.toml")
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| cwd.to_path_buf());
    Ok(ResolvedCommand {
        pyproject,
        profile,
        files: resolved_files,
        project_root,
    })
}

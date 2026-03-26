use std::path::PathBuf;

use tracing::debug;

use crate::args::{FileSelectionArgs, Profile};
use crate::error::Result;
use crate::pyproject::{PyprojectSettings, load_options};
use crate::resolver::{ResolvedDiscoveryConfig, resolve_files};

pub mod check;
pub mod format;

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
    let pyproject = std::env::current_dir().map_or_else(
        |err| {
            debug!("Failed to get current directory: {err}");
            PyprojectSettings::default()
        },
        load_options,
    );
    let profile = profile.or(pyproject.profile);
    let discovery_config = ResolvedDiscoveryConfig::new(file_selection, &pyproject);
    let resolved_files = resolve_files(files, &discovery_config)?;
    Ok(ResolvedCommand {
        pyproject,
        profile,
        files: resolved_files,
    })
}

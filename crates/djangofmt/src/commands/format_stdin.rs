use std::io::{Write, stdout};
use std::path::Path;

use tracing::{debug, error};

use crate::ExitStatus;
use crate::args::{FormatCommand, Profile};
use crate::commands::format::{FormatterConfig, format_text};
use crate::error::{CommandError, ParseError, Result};
use crate::pyproject::{PyprojectSettings, load_options};
use crate::resolver::{ResolvedDiscoveryConfig, is_force_excluded};
use crate::stdin::{parrot_stdin, read_from_stdin};

/// Run the formatter over a single file, read from `stdin`.
pub fn format_stdin(cli: &FormatCommand) -> Result<ExitStatus> {
    let pyproject = std::env::current_dir().map_or_else(
        |err| {
            debug!("Failed to get current directory: {err}");
            PyprojectSettings::default()
        },
        load_options,
    );
    let discovery_config = ResolvedDiscoveryConfig::new(&cli.file_selection, &pyproject);

    // If force-exclude is enabled and the (virtual) stdin filename matches an exclude
    // pattern, parrot the input back to stdout unchanged.
    if let Some(filename) = cli.stdin_filename.as_deref()
        && is_force_excluded(filename, &discovery_config)?
    {
        parrot_stdin()?;
        return Ok(ExitStatus::Success);
    }

    let config = FormatterConfig::from_args(cli, &pyproject);
    let profile = cli
        .profile
        .or_else(|| cli.stdin_filename.as_deref().and_then(Profile::from_path))
        .or(pyproject.profile)
        .unwrap_or_default();

    match format_source_code(cli.stdin_filename.as_deref(), &config, profile) {
        Ok(()) => Ok(ExitStatus::Success),
        Err(err) => {
            error!("{:?}", miette::Report::new(*err));
            Ok(ExitStatus::Error)
        }
    }
}

/// Format source code read from `stdin` and write the result to `stdout`.
fn format_source_code(
    path: Option<&Path>,
    config: &FormatterConfig,
    profile: Profile,
) -> std::result::Result<(), Box<CommandError>> {
    let source = read_from_stdin()
        .map_err(|err| Box::new(CommandError::Read(path.map(Path::to_path_buf), err)))?;

    let formatted = match format_text(&source, config, profile) {
        Ok(f) => f,
        Err(err) => {
            return Err(Box::new(CommandError::Parse(ParseError::new(
                path.map(Path::to_path_buf),
                source,
                &err,
            ))));
        }
    };

    let output = formatted.as_deref().unwrap_or(&source);
    let mut writer = stdout().lock();
    writer
        .write_all(output.as_bytes())
        .map_err(|err| Box::new(CommandError::Write(path.map(Path::to_path_buf), err)))?;
    Ok(())
}

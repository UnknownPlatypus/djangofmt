use std::io::{Read, Write, stdin, stdout};
use std::path::Path;

use tracing::error;

use crate::ExitStatus;
use crate::args::{FormatCommand, Profile};
use crate::commands::format::{FormatterConfig, format_text};
use crate::error::{CommandError, ParseError, Result};
use crate::pyproject::load_pyproject_from_cwd;
use crate::resolver::{ResolvedDiscoveryConfig, is_force_excluded};

/// Run the formatter over a single file, read from `stdin`.
pub fn format_stdin(cli: &FormatCommand) -> Result<ExitStatus> {
    let stdin_filename = cli.stdin_filename.as_deref();
    let pyproject = load_pyproject_from_cwd();
    let discovery_config = ResolvedDiscoveryConfig::new(&cli.file_selection, &pyproject);

    // If force-exclude matches the (virtual) stdin filename, parrot stdin to
    // stdout unchanged so editors don't trip on excluded files.
    if let Some(filename) = stdin_filename
        && is_force_excluded(filename, &discovery_config)?
    {
        std::io::copy(&mut stdin().lock(), &mut stdout().lock())?;
        return Ok(ExitStatus::Success);
    }

    let config = FormatterConfig::from_args(cli, &pyproject);
    let profile = cli
        .profile
        .or_else(|| stdin_filename.and_then(Profile::from_path))
        .or(pyproject.profile)
        .unwrap_or_default();

    match format_source_code(stdin_filename, &config, profile) {
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
    let mut source = String::new();
    stdin()
        .lock()
        .read_to_string(&mut source)
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
    stdout()
        .lock()
        .write_all(output.as_bytes())
        .map_err(|err| Box::new(CommandError::Write(path.map(Path::to_path_buf), err)))?;
    Ok(())
}

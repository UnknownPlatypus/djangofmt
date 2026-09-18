use std::io::{stdin, stdout};

use crate::ExitStatus;
use crate::args::CheckCommand;
use crate::commands::check::{CheckRun, Source, check_source};
use crate::config::resolve_profile;
use crate::error::Result;
use crate::fs::normalize_path;
use crate::pyproject::load_pyproject_from_cwd;
use crate::resolver::{ResolvedDiscoveryConfig, is_force_excluded};

/// Run the linter over a single template, read from `stdin`.
pub fn check_stdin(cli: &CheckCommand) -> Result<ExitStatus> {
    // Like ruff, normalize to an absolute path so it lines up with the discovered files
    // that `per-file-ignores` globs are anchored against.
    let stdin_filename = cli.stdin_filename.as_deref().map(normalize_path);
    let stdin_filename = stdin_filename.as_deref();
    let (pyproject, project_root) = load_pyproject_from_cwd()?;
    let run = CheckRun::new(cli, &pyproject, &project_root)?;

    // If force-exclude matches the (virtual) stdin filename, leave it alone: `--fix`
    // parrots stdin to stdout unchanged so editors don't trip on excluded files.
    let discovery_config =
        ResolvedDiscoveryConfig::new(&cli.file_selection, &pyproject, &project_root);
    if let Some(filename) = stdin_filename
        && is_force_excluded(filename, &discovery_config)?
    {
        if run.fix.is_some() {
            std::io::copy(&mut stdin().lock(), &mut stdout().lock())?;
        }
        return Ok(ExitStatus::Success);
    }

    let settings = run.settings_for(stdin_filename);
    let profile = resolve_profile(cli.template.profile, pyproject.profile, stdin_filename);
    let outcome = super::catch_file_panic(stdin_filename, || {
        check_source(
            Source::Stdin(stdin_filename),
            profile,
            &settings,
            &run.custom_blocks,
            run.fix,
        )
    });
    let (results, errors) = match outcome {
        Ok(result) => (vec![result], Vec::new()),
        Err(err) => (Vec::new(), vec![*err]),
    };
    Ok(run.report(&results, errors))
}

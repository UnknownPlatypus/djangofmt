use djangofmt_lint::{
    Applicability, FileDiagnostics, FixerError, RuleFixSummary, Settings, check_ast, lint_fix,
};
use markup_fmt::FormatError;
use markup_fmt::parser::Parser;
use miette::NamedSource;
use rayon::iter::Either::{Left, Right};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::FxHashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, error, info};

use crate::ExitStatus;
use crate::args::{CheckCommand, Profile};
use crate::error::{CommandError, ParseError, Result};
use crate::fs::relativize_path;

/// Per-file outcome of `check_path`.
struct CheckResult {
    /// Owning path for display.
    path: PathBuf,
    /// Diagnostics still present after any fixes were applied.
    file_diagnostics: FileDiagnostics,
    /// Total fixes applied to this file (0 when `--fix` is off).
    applied_count: usize,
    /// Per-rule applied summaries, for `--show-fixes`.
    fixes_by_rule: FxHashMap<String, RuleFixSummary>,
}

/// Check the given source code for linting errors.
pub fn check(args: &CheckCommand) -> Result<ExitStatus> {
    let resolved = super::resolve_command(&args.files, args.profile, &args.file_selection)?;

    let settings = Settings::default();
    let threshold = if args.unsafe_fixes {
        Applicability::Unsafe
    } else {
        Applicability::Safe
    };

    let start = Instant::now();
    let (results, mut parse_errors): (Vec<_>, Vec<_>) = resolved
        .files
        .par_iter()
        .map(|path| check_path(path, resolved.profile, &settings, args.fix, threshold))
        .partition_map(|result| match result {
            Ok(r) => Left(r),
            Err(err) => Right(err),
        });

    let duration = start.elapsed();
    debug!("Checked {} files in {:.2?}", resolved.files.len(), duration);

    parse_errors.sort_unstable_by(|a, b| a.path().cmp(&b.path()));
    let error_count = parse_errors.len();
    for err in parse_errors {
        error!("{:?}", miette::Report::new(*err));
    }
    if error_count > 0 {
        error!("Couldn't check {} files!", error_count);
    }

    let mut total_diagnostics = 0usize;
    let mut total_applied = 0usize;
    let mut total_safe_fixable = 0usize;
    let mut total_unsafe_fixable = 0usize;
    for result in &results {
        total_diagnostics += result.file_diagnostics.len();
        total_applied += result.applied_count;
        for diag in &result.file_diagnostics.related {
            let Some(fix) = diag.fix.as_ref() else {
                continue;
            };
            if fix.applies(Applicability::Safe) {
                total_safe_fixable += 1;
            } else if fix.applies(Applicability::Unsafe) {
                total_unsafe_fixable += 1;
            }
        }
    }
    if args.unsafe_fixes {
        total_unsafe_fixable = 0;
    }

    for result in &results {
        if !result.file_diagnostics.is_empty() {
            error!("{:?}", miette::Report::new(result.file_diagnostics.clone()));
        }
    }

    print_summary(
        total_diagnostics,
        total_applied,
        total_safe_fixable,
        total_unsafe_fixable,
        args.fix,
        error_count,
    );

    if args.show_fixes && total_applied > 0 {
        print_show_fixes(&results, total_applied);
    }

    if total_diagnostics == 0 && error_count == 0 {
        return Ok(ExitStatus::Success);
    }
    Ok(ExitStatus::Failure)
}

fn print_summary(
    total: usize,
    applied: usize,
    safe_fixable: usize,
    unsafe_fixable: usize,
    apply_to_disk: bool,
    parse_errors: usize,
) {
    if total == 0 && applied == 0 {
        if parse_errors == 0 {
            info!("All checks passed!");
        }
        return;
    }

    if apply_to_disk {
        let found = applied + total;
        info!("Found {found} errors ({applied} fixed, {total} remaining).");
    } else if safe_fixable > 0 {
        let suffix = if unsafe_fixable > 0 {
            format!(" ({unsafe_fixable} hidden fixes can be enabled with --unsafe-fixes)")
        } else {
            String::new()
        };
        info!("Found {total} errors. [*] {safe_fixable} fixable with the --fix option.{suffix}");
    } else if unsafe_fixable > 0 {
        info!(
            "Found {total} errors. ({unsafe_fixable} hidden fixes can be enabled with --unsafe-fixes)"
        );
    } else {
        info!("Found {total} errors.");
    }
}

fn print_show_fixes(results: &[CheckResult], total_applied: usize) {
    info!("Fixed {total_applied} errors:");
    for result in results {
        if result.applied_count == 0 {
            continue;
        }
        info!("- {}:", relativize_path(&result.path));
        let mut entries: Vec<_> = result.fixes_by_rule.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        for (rule, summary) in entries {
            let count = summary.count;
            if let Some(title) = summary.fix_title.as_deref() {
                info!("    {count} × {rule} ({title})");
            } else {
                info!("    {count} × {rule}");
            }
        }
    }
}

/// Check the file at the given [`Path`] for linting issues.
#[tracing::instrument(
    level = "debug",
    skip_all,
    fields(path = %path.display())
)]
fn check_path(
    path: &Path,
    profile: Option<Profile>,
    settings: &Settings,
    fix: bool,
    threshold: Applicability,
) -> std::result::Result<CheckResult, Box<CommandError>> {
    let profile = profile
        .or_else(|| Profile::from_path(path))
        .unwrap_or_default();
    let source = fs::read_to_string(path)
        .map_err(|err| CommandError::Read(Some(path.to_path_buf()), err))?;

    let mut parser = Parser::new(&source, profile.into(), vec![]);
    let ast = match parser.parse_root() {
        Ok(ast) => ast,
        Err(err) => {
            return Err(Box::new(CommandError::Parse(ParseError::new(
                Some(path.to_path_buf()),
                source,
                &FormatError::<markup_fmt::SyntaxError>::Syntax(err),
            ))));
        }
    };

    if fix {
        match lint_fix(&source, settings, profile.into(), threshold) {
            Ok(result) => {
                if result.applied_count > 0 && result.source != source {
                    fs::write(path, &result.source)
                        .map_err(|err| CommandError::Write(Some(path.to_path_buf()), err))?;
                }

                let file_diagnostics = if result.remaining_diagnostics.is_empty() {
                    FileDiagnostics::empty()
                } else {
                    FileDiagnostics::new(
                        NamedSource::new(relativize_path(path), result.source),
                        result.remaining_diagnostics,
                    )
                };

                return Ok(CheckResult {
                    path: path.to_path_buf(),
                    file_diagnostics,
                    applied_count: result.applied_count,
                    fixes_by_rule: result.applied_by_rule,
                });
            }
            Err(FixerError::SyntaxRegression {
                iteration,
                error: _,
            }) => {
                error!(
                    "Fix introduced a syntax error in {} at iteration {iteration}, leaving file unchanged",
                    path.display()
                );
                // Fall through to the no-fix path on the original AST.
            }
        }
    }

    let diagnostics = check_ast(&source, &ast, settings);
    let file_diagnostics = if diagnostics.is_empty() {
        FileDiagnostics::empty()
    } else {
        FileDiagnostics::new(NamedSource::new(relativize_path(path), source), diagnostics)
    };

    Ok(CheckResult {
        path: path.to_path_buf(),
        file_diagnostics,
        applied_count: 0,
        fixes_by_rule: FxHashMap::default(),
    })
}

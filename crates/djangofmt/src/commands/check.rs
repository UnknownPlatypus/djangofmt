use djangofmt_lint::{Applicability, FileDiagnostics, RuleFixSummary, Settings, lint_text};
use markup_fmt::FormatError;
use miette::{SourceCode, SpanContents};
use rayon::iter::Either::{Left, Right};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::fmt::Write;
use std::fs;
use std::io::{self, Write as _};
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error, info, warn};

use crate::args::{CheckCommand, OutputFormat, Profile};
use crate::config::{resolve_bool_arg, resolve_lint_configuration, resolve_profile};
use crate::error::{CommandError, ParseError, Result};
use crate::fs::relativize_path;
use crate::per_file_ignores::PerFileIgnores;
use crate::pyproject::{LintSettings, PyprojectSettings};
use crate::{ExitStatus, STDIN_SENTINEL};

use super::format::merge_custom_blocks;

/// Resolved fix-related configuration after merging CLI args with pyproject settings.
#[derive(Debug, PartialEq, Eq)]
pub struct CheckConfig {
    pub fix: bool,
    pub unsafe_fixes: bool,
    pub show_fixes: bool,
    pub output_format: OutputFormat,
}

impl CheckConfig {
    /// Build a [`CheckConfig`] by merging CLI arguments with `[tool.djangofmt.lint]` settings.
    ///
    /// CLI arguments take precedence over pyproject settings, which take precedence over defaults.
    #[must_use]
    pub fn from_args(args: &CheckCommand, lint: Option<&LintSettings>) -> Self {
        let default = LintSettings::default();
        let lint = lint.unwrap_or(&default);
        Self {
            fix: resolve_bool_arg(args.fix, args.no_fix)
                .or(lint.fix)
                .unwrap_or_default(),
            unsafe_fixes: resolve_bool_arg(args.unsafe_fixes, args.no_unsafe_fixes)
                .or(lint.unsafe_fixes)
                .unwrap_or_default(),
            show_fixes: resolve_bool_arg(args.show_fixes, args.no_show_fixes)
                .or(lint.show_fixes)
                .unwrap_or_default(),
            output_format: args
                .output_format
                .or(lint.output_format)
                .unwrap_or_default(),
        }
    }

    /// How far a fix may go before it is held back.
    const fn threshold(&self) -> Applicability {
        if self.unsafe_fixes {
            Applicability::Unsafe
        } else {
            Applicability::Safe
        }
    }
}

/// Per-file outcome of `check_source`.
pub(crate) struct CheckResult {
    /// Relativized path, or `-` for stdin.
    display_path: String,
    /// Diagnostics still present after any fixes were applied.
    file_diagnostics: FileDiagnostics,
    /// Total fixes applied to this file (0 when `--fix` is off).
    applied_count: usize,
    /// Per-rule applied summaries, for `--show-fixes`.
    fixes_by_rule: FxHashMap<&'static str, RuleFixSummary>,
    /// Whether the file was skipped via `file-ignore[invalid-syntax]`.
    skipped: bool,
}

/// Aggregate counts behind the run summary.
#[derive(Default)]
struct Totals {
    diagnostics: usize,
    applied: usize,
    safe_fixable: usize,
    unsafe_fixable: usize,
    skipped: usize,
}

impl Totals {
    fn of(results: &[CheckResult]) -> Self {
        let mut totals = Self::default();
        for result in results {
            totals.diagnostics += result.file_diagnostics.len();
            totals.applied += result.applied_count;
            totals.skipped += usize::from(result.skipped);
            for diag in &result.file_diagnostics.diagnostics {
                let Some(fix) = diag.fix.as_ref() else {
                    continue;
                };
                if fix.applies(Applicability::Safe) {
                    totals.safe_fixable += 1;
                } else if fix.applies(Applicability::Unsafe) {
                    totals.unsafe_fixable += 1;
                }
            }
        }
        totals
    }
}

/// What `check` and `check_stdin` share once the pyproject is loaded.
pub(crate) struct CheckRun {
    config: CheckConfig,
    settings: Settings,
    per_file_ignores: Option<PerFileIgnores>,
    /// Same custom blocks as `format`, so both commands lint/format the same AST.
    pub(crate) custom_blocks: Vec<String>,
    /// One value carries both "should we fix" and "how far", so they can't disagree.
    pub(crate) fix: Option<Applicability>,
}

impl CheckRun {
    pub(crate) fn new(
        args: &CheckCommand,
        pyproject: &PyprojectSettings,
        project_root: &Path,
    ) -> Result<Self> {
        let lint = pyproject.lint.as_ref();
        let config = CheckConfig::from_args(args, lint);

        let (settings, warnings) = resolve_lint_configuration(args, lint).into_settings();
        for warning in &warnings {
            warn!("{warning}");
        }

        let per_file_ignores = lint
            .and_then(|l| l.per_file_ignores.as_ref())
            .map(|patterns| PerFileIgnores::new(patterns, project_root))
            .transpose()?;

        let fix = config.fix.then_some(config.threshold());

        let custom_blocks = merge_custom_blocks(
            args.template.custom_blocks.clone(),
            pyproject.custom_blocks.clone(),
        )
        .unwrap_or_default();

        Ok(Self {
            config,
            settings,
            per_file_ignores,
            custom_blocks,
            fix,
        })
    }

    /// The run's settings, narrowed by `per-file-ignores` when they match `path`.
    pub(crate) fn settings_for(&self, path: Option<&Path>) -> Cow<'_, Settings> {
        match (&self.per_file_ignores, path) {
            (Some(pfi), Some(path)) => Cow::Owned(Settings {
                rules: pfi.rules_for(path, &self.settings.rules),
                ..self.settings.clone()
            }),
            _ => Cow::Borrowed(&self.settings),
        }
    }

    /// Print diagnostics, errors and the summary, then fold them into the exit code.
    pub(crate) fn report(&self, results: &[CheckResult], errors: Vec<CommandError>) -> ExitStatus {
        let nb_errors = super::report_errors(errors, "check", self.config.output_format);

        let mut totals = Totals::of(results);
        if self.config.fix && self.config.unsafe_fixes {
            totals.unsafe_fixable = 0;
        }

        match self.config.output_format {
            OutputFormat::Full => print_full(results),
            OutputFormat::Concise => print_concise(results, self.config.threshold()),
        }

        print_summary(
            totals.diagnostics,
            totals.applied,
            totals.safe_fixable,
            totals.unsafe_fixable,
            self.config.fix,
            self.config.unsafe_fixes,
            nb_errors,
        );

        print_skipped(totals.skipped);

        if self.config.show_fixes && totals.applied > 0 {
            print_show_fixes(results, totals.applied);
        }

        // I/O, parse and panic errors take precedence over lint violations in the exit code.
        if nb_errors > 0 {
            return ExitStatus::Error;
        }
        if totals.diagnostics > 0 {
            return ExitStatus::Failure;
        }
        ExitStatus::Success
    }
}

/// Check the given files for linting errors.
pub fn check(args: &CheckCommand) -> Result<ExitStatus> {
    let resolved = super::resolve_command(&args.files, &args.file_selection)?;
    let run = CheckRun::new(args, &resolved.pyproject, &resolved.project_root)?;

    let start = Instant::now();
    let (results, errors): (Vec<_>, Vec<_>) = resolved
        .files
        .par_iter()
        .map(|path| {
            let settings = run.settings_for(Some(path));
            let profile = resolve_profile(
                args.template.profile,
                resolved.pyproject.profile,
                Some(path),
            );
            super::catch_file_panic(Some(path), || {
                check_source(
                    Source::File(path),
                    profile,
                    &settings,
                    &run.custom_blocks,
                    run.fix,
                )
            })
        })
        .partition_map(|result| match result {
            Ok(r) => Left(r),
            Err(err) => Right(*err),
        });

    let duration = start.elapsed();
    debug!("Checked {} files in {:.2?}", resolved.files.len(), duration);

    Ok(run.report(&results, errors))
}

/// Render each diagnostic as its own block, with source snippet and help text.
fn print_full(results: &[CheckResult]) {
    for result in results {
        if result.file_diagnostics.is_empty() {
            continue;
        }
        // One record per file: `error!` adds its own newline, so rendering each
        // diagnostic separately would double the blank line between blocks.
        let mut rendered = String::new();
        for report in result.file_diagnostics.reports() {
            write!(rendered, "{report:?}").expect("rendering to a String cannot fail");
        }
        error!("{}", rendered.trim_start_matches('\n'));
    }
}

/// Render one `path:line:col: rule [*] message` line per diagnostic.
fn print_concise(results: &[CheckResult], threshold: Applicability) {
    for result in results {
        let source = &result.file_diagnostics.source_code;
        let path = &result.display_path;
        for diag in &result.file_diagnostics.diagnostics {
            let (line, column) = source
                .read_span(&diag.span, 0, 0)
                .map_or((0, 0), |contents| {
                    (contents.line() + 1, contents.column() + 1)
                });
            let fixable = if diag.fix.as_ref().is_some_and(|fix| fix.applies(threshold)) {
                " [*]"
            } else {
                ""
            };
            error!(
                "{path}:{line}:{column}: {}{fixable} {}",
                diag.code, diag.message
            );
        }
    }
}

fn print_summary(
    total: usize,
    applied: usize,
    safe_fixable: usize,
    unsafe_fixable: usize,
    apply_to_disk: bool,
    unsafe_fixes_enabled: bool,
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
        return;
    }

    // With `--unsafe-fixes` set (but no `--fix`), unsafe fixes count toward
    // what `--fix` would apply; without it they are reported as hidden.
    let fixable_with_fix = safe_fixable
        + if unsafe_fixes_enabled {
            unsafe_fixable
        } else {
            0
        };
    let hidden = if unsafe_fixes_enabled {
        0
    } else {
        unsafe_fixable
    };

    if fixable_with_fix > 0 {
        let suffix = if hidden > 0 {
            format!(" ({hidden} hidden fixes can be enabled with --unsafe-fixes)")
        } else {
            String::new()
        };
        info!(
            "Found {total} errors. [*] {fixable_with_fix} fixable with the --fix option.{suffix}"
        );
    } else if hidden > 0 {
        info!("Found {total} errors. ({hidden} hidden fixes can be enabled with --unsafe-fixes)");
    } else {
        info!("Found {total} errors.");
    }
}

/// Mirror the format command's skip counter for quarantined files.
fn print_skipped(count: usize) {
    if count > 0 {
        info!(
            "{count} file{} skipped !",
            if count == 1 { "" } else { "s" }
        );
    }
}

fn print_show_fixes(results: &[CheckResult], total_applied: usize) {
    info!("Fixed {total_applied} errors:");
    for result in results {
        if result.applied_count == 0 {
            continue;
        }
        info!("- {}:", result.display_path);
        let mut entries: Vec<_> = result.fixes_by_rule.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        for (rule, summary) in entries {
            let count = summary.count;
            if let Some(title) = summary.fix_title {
                info!("    {count} × {rule} ({title})");
            } else {
                info!("    {count} × {rule}");
            }
        }
    }
}

/// Where a checked template is read from, and where its fixed text is written back to.
#[derive(Clone, Copy)]
pub(crate) enum Source<'a> {
    File(&'a Path),
    /// Standard input, named through `--stdin-filename` when the editor knows the path.
    Stdin(Option<&'a Path>),
}

impl<'a> Source<'a> {
    const fn path(self) -> Option<&'a Path> {
        match self {
            Self::File(path) => Some(path),
            Self::Stdin(path) => path,
        }
    }

    fn read(self) -> std::result::Result<String, Box<CommandError>> {
        match self {
            Self::File(path) => fs::read_to_string(path),
            Self::Stdin(_) => io::read_to_string(io::stdin().lock()),
        }
        .map_err(|err| CommandError::Read(self.path().map(Path::to_path_buf), err).into())
    }
}

/// Check one template for linting issues, applying fixes when requested.
#[tracing::instrument(
    level = "debug",
    skip_all,
    fields(path = ?source.path())
)]
pub(crate) fn check_source(
    source: Source<'_>,
    profile: Profile,
    settings: &Settings,
    custom_blocks: &[String],
    fix: Option<Applicability>,
) -> std::result::Result<CheckResult, Box<CommandError>> {
    let path = source.path();
    let display_path = path.map_or_else(|| STDIN_SENTINEL.to_owned(), relativize_path);
    let text = source.read()?;

    let outcome = lint_text(&text, settings, profile.into(), custom_blocks, fix, path);

    // Like ruff, `--fix` on stdin always echoes the source (fixed, unchanged or even
    // unparsable) so editors piping it back never end up with an empty buffer.
    if matches!(source, Source::Stdin(_)) && fix.is_some() {
        let echoed = outcome
            .as_ref()
            .ok()
            .and_then(|outcome| outcome.as_ref()?.fixed.as_deref())
            .unwrap_or(&text);
        io::stdout()
            .lock()
            .write_all(echoed.as_bytes())
            .map_err(|err| CommandError::Write(path.map(Path::to_path_buf), err))?;
    }

    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(err) => {
            return Err(ParseError::new(
                path.map(Path::to_path_buf),
                text,
                &FormatError::Syntax(err),
            )
            .into());
        }
    };
    let skipped = outcome.is_none();
    if skipped {
        debug!("Skipping {display_path} (file-ignore[invalid-syntax])");
    }
    let outcome = outcome.unwrap_or_default();

    if let (Source::File(path), Some(fixed)) = (source, &outcome.fixed) {
        fs::write(path, fixed).map_err(|err| CommandError::Write(Some(path.to_path_buf()), err))?;
    }
    let text = outcome.fixed.unwrap_or(text);

    let file_diagnostics = if outcome.diagnostics.is_empty() {
        FileDiagnostics::empty()
    } else {
        FileDiagnostics::new(&display_path, text, outcome.diagnostics)
    };

    Ok(CheckResult {
        display_path,
        file_diagnostics,
        applied_count: outcome.applied_count,
        fixes_by_rule: outcome.applied_by_rule,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::{CheckConfig, print_summary};
    use crate::args::{CheckCommand, OutputFormat};
    use crate::pyproject::LintSettings;
    use tracing_test::traced_test;

    #[test]
    fn check_config_defaults_to_false() {
        let config = CheckConfig::from_args(&CheckCommand::default(), None);
        assert_eq!(
            config,
            CheckConfig {
                fix: false,
                unsafe_fixes: false,
                show_fixes: false,
                output_format: OutputFormat::Full,
            }
        );
    }

    #[test]
    fn check_config_reads_pyproject_settings() {
        let lint = LintSettings {
            fix: Some(true),
            unsafe_fixes: Some(true),
            show_fixes: Some(true),
            output_format: Some(OutputFormat::Concise),
            ..Default::default()
        };
        let config = CheckConfig::from_args(&CheckCommand::default(), Some(&lint));
        assert_eq!(
            config,
            CheckConfig {
                fix: true,
                unsafe_fixes: true,
                show_fixes: true,
                output_format: OutputFormat::Concise,
            }
        );
    }

    #[test]
    fn check_config_cli_yes_overrides_pyproject() {
        let lint = LintSettings {
            fix: Some(false),
            unsafe_fixes: Some(false),
            show_fixes: Some(false),
            output_format: Some(OutputFormat::Concise),
            ..Default::default()
        };
        let args = CheckCommand {
            fix: true,
            unsafe_fixes: true,
            show_fixes: true,
            output_format: Some(OutputFormat::Full),
            ..Default::default()
        };
        let config = CheckConfig::from_args(&args, Some(&lint));
        assert_eq!(config.output_format, OutputFormat::Full);
        assert!(config.fix);
        assert!(config.unsafe_fixes);
        assert!(config.show_fixes);
    }

    #[test]
    fn check_config_cli_no_overrides_pyproject() {
        let lint = LintSettings {
            fix: Some(true),
            unsafe_fixes: Some(true),
            show_fixes: Some(true),
            ..Default::default()
        };
        let args = CheckCommand {
            no_fix: true,
            no_unsafe_fixes: true,
            no_show_fixes: true,
            ..Default::default()
        };
        let config = CheckConfig::from_args(&args, Some(&lint));
        assert!(!config.fix);
        assert!(!config.unsafe_fixes);
        assert!(!config.show_fixes);
    }

    #[test]
    #[traced_test]
    fn summary_all_passed() {
        print_summary(0, 0, 0, 0, false, false, 0);
        assert!(logs_contain("All checks passed!"));
    }

    #[test]
    #[traced_test]
    fn summary_silent_when_only_parse_errors() {
        print_summary(0, 0, 0, 0, false, false, 2);
        assert!(!logs_contain("All checks passed!"));
        assert!(!logs_contain("Found"));
    }

    #[test]
    #[traced_test]
    fn summary_apply_to_disk_ignores_fixable_counts() {
        // Under `--fix`, fixable counts shouldn't leak into the message.
        print_summary(2, 3, 4, 5, true, true, 0);
        assert!(logs_contain("Found 5 errors (3 fixed, 2 remaining)."));
        assert!(!logs_contain("fixable with"));
        assert!(!logs_contain("hidden"));
    }

    #[test]
    #[traced_test]
    fn summary_check_only_safe_fixable() {
        print_summary(7, 0, 2, 0, false, false, 0);
        assert!(logs_contain(
            "Found 7 errors. [*] 2 fixable with the --fix option."
        ));
        assert!(!logs_contain("hidden"));
    }

    #[test]
    #[traced_test]
    fn summary_check_only_safe_and_unsafe_hidden() {
        print_summary(7, 0, 2, 3, false, false, 0);
        assert!(logs_contain(
            "Found 7 errors. [*] 2 fixable with the --fix option. \
             (3 hidden fixes can be enabled with --unsafe-fixes)"
        ));
    }

    #[test]
    #[traced_test]
    fn summary_check_only_unsafe_hidden() {
        print_summary(7, 0, 0, 3, false, false, 0);
        assert!(logs_contain(
            "Found 7 errors. (3 hidden fixes can be enabled with --unsafe-fixes)"
        ));
        assert!(!logs_contain("fixable with"));
    }

    #[test]
    #[traced_test]
    fn summary_check_with_unsafe_fixes_enabled_combines_counts() {
        // `--unsafe-fixes` set, but no `--fix`: unsafe fixes are now reportable
        // as fixable, not hidden.
        print_summary(7, 0, 2, 3, false, true, 0);
        assert!(logs_contain(
            "Found 7 errors. [*] 5 fixable with the --fix option."
        ));
        assert!(!logs_contain("hidden"));
    }

    #[test]
    #[traced_test]
    fn summary_check_with_unsafe_fixes_enabled_only_unsafe() {
        print_summary(7, 0, 0, 3, false, true, 0);
        assert!(logs_contain(
            "Found 7 errors. [*] 3 fixable with the --fix option."
        ));
        assert!(!logs_contain("hidden"));
    }

    #[test]
    #[traced_test]
    fn summary_check_no_fixes_available() {
        print_summary(5, 0, 0, 0, false, false, 0);
        assert!(logs_contain("Found 5 errors."));
        assert!(!logs_contain("fixable with"));
        assert!(!logs_contain("hidden"));
    }
}

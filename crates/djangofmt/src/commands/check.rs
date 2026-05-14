use djangofmt_lint::{
    Applicability, FileDiagnostics, FixerError, PreviewMode, RuleFixSummary, RuleSelection,
    RuleSelector, Settings, check_ast, lint_fix,
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
use crate::pyproject::{LintTomlSettings, PyprojectSettings};
use crate::resolver::resolve_bool_arg;

/// Resolved fix-related configuration after merging CLI args with pyproject settings.
#[derive(Debug, PartialEq, Eq)]
pub struct CheckConfig {
    pub fix: bool,
    pub unsafe_fixes: bool,
    pub show_fixes: bool,
}

impl CheckConfig {
    /// Build a [`CheckConfig`] by merging CLI arguments with pyproject.toml settings.
    ///
    /// CLI arguments take precedence over pyproject settings, which take precedence over defaults.
    #[must_use]
    pub fn from_args(args: &CheckCommand, pyproject: &PyprojectSettings) -> Self {
        Self {
            fix: resolve_bool_arg(args.fix, args.no_fix)
                .or(pyproject.fix)
                .unwrap_or_default(),
            unsafe_fixes: resolve_bool_arg(args.unsafe_fixes, args.no_unsafe_fixes)
                .or(pyproject.unsafe_fixes)
                .unwrap_or_default(),
            show_fixes: resolve_bool_arg(args.show_fixes, args.no_show_fixes)
                .or(pyproject.show_fixes)
                .unwrap_or_default(),
        }
    }
}

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
    let config = CheckConfig::from_args(args, &resolved.pyproject);

    let settings = resolve_settings(args, &resolved.pyproject);
    let threshold = if config.unsafe_fixes {
        Applicability::Unsafe
    } else {
        Applicability::Safe
    };

    let start = Instant::now();
    let (results, mut parse_errors): (Vec<_>, Vec<_>) = resolved
        .files
        .par_iter()
        .map(|path| check_path(path, resolved.profile, &settings, config.fix, threshold))
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
    if config.fix && config.unsafe_fixes {
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
        config.fix,
        config.unsafe_fixes,
        error_count,
    );

    if config.show_fixes && total_applied > 0 {
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

/// Resolve the linter [`Settings`] from CLI args and pyproject options.
///
/// Builds two [`RuleSelection`] layers (pyproject < CLI) and hands them to
/// [`Settings::from_selections`], which prepends the implicit `select=[ALL]`
/// base layer and folds them in order. A layer with `Some(select)` replaces
/// the running set (this is what makes `--select=ALL` on the CLI cleanly
/// override `[tool.djangofmt.lint] ignore = [...]` in pyproject), while a
/// layer with `select == None` extends the running set additively.
///
/// `PreviewMode` precedence is `--preview` > `--no-preview` > pyproject >
/// disabled.
fn resolve_settings(cli: &CheckCommand, pyproject: &PyprojectSettings) -> Settings {
    let pyproject_layer = Layer::from_pyproject(pyproject.lint.as_ref());
    let cli_layer = Layer::from_cli(cli);
    let selections = [pyproject_layer.as_selection(), cli_layer.as_selection()];

    Settings::from_selections(&selections, resolve_preview(cli, pyproject))
}

/// Resolve `PreviewMode` from CLI flags and pyproject options.
fn resolve_preview(cli: &CheckCommand, pyproject: &PyprojectSettings) -> PreviewMode {
    if let Some(enabled) = resolve_bool_arg(cli.preview, cli.no_preview) {
        return PreviewMode::from(enabled);
    }
    pyproject
        .lint
        .as_ref()
        .and_then(|l| l.preview)
        .map_or(PreviewMode::Disabled, PreviewMode::from)
}

/// Borrowed view of a single rule-selection layer.
///
/// Mirrors `LintTomlSettings` / `CheckCommand`'s rule-selection fields with
/// uniform `Option<&[...]>` slots, ready to be converted to a
/// [`RuleSelection`] for [`Settings::from_selections`].
struct Layer<'a> {
    select: Option<&'a [RuleSelector]>,
    ignore: Option<&'a [RuleSelector]>,
    extend_select: Option<&'a [RuleSelector]>,
    extend_ignore: Option<&'a [RuleSelector]>,
}

impl<'a> Layer<'a> {
    fn from_pyproject(lint: Option<&'a LintTomlSettings>) -> Self {
        lint.map_or_else(Self::empty, |l| Self {
            select: l.select.as_deref(),
            ignore: l.ignore.as_deref(),
            extend_select: l.extend_select.as_deref(),
            extend_ignore: l.extend_ignore.as_deref(),
        })
    }

    fn from_cli(cli: &'a CheckCommand) -> Self {
        Self {
            select: cli.select.as_deref(),
            ignore: cli.ignore.as_deref(),
            extend_select: cli.extend_select.as_deref(),
            extend_ignore: cli.extend_ignore.as_deref(),
        }
    }

    const fn empty() -> Self {
        Self {
            select: None,
            ignore: None,
            extend_select: None,
            extend_ignore: None,
        }
    }

    /// Convert to a [`RuleSelection`] for the lint crate's resolver.
    fn as_selection(&self) -> RuleSelection<'a> {
        RuleSelection {
            select: self.select,
            ignore: self.ignore.unwrap_or(&[]),
            extend_select: self.extend_select.unwrap_or(&[]),
            extend_ignore: self.extend_ignore.unwrap_or(&[]),
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
                &FormatError::Syntax(err),
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

#[cfg(test)]
mod tests {
    use super::*;
    use djangofmt_lint::{Rule, RuleCategory};
    use tracing_test::traced_test;

    #[test]
    fn check_config_defaults_to_false() {
        let config =
            CheckConfig::from_args(&CheckCommand::default(), &PyprojectSettings::default());
        assert_eq!(
            config,
            CheckConfig {
                fix: false,
                unsafe_fixes: false,
                show_fixes: false,
            }
        );
    }

    #[test]
    fn check_config_reads_pyproject_settings() {
        let pyproject = PyprojectSettings {
            fix: Some(true),
            unsafe_fixes: Some(true),
            show_fixes: Some(true),
            ..Default::default()
        };
        let config = CheckConfig::from_args(&CheckCommand::default(), &pyproject);
        assert_eq!(
            config,
            CheckConfig {
                fix: true,
                unsafe_fixes: true,
                show_fixes: true,
            }
        );
    }

    #[test]
    fn check_config_cli_yes_overrides_pyproject() {
        let pyproject = PyprojectSettings {
            fix: Some(false),
            unsafe_fixes: Some(false),
            show_fixes: Some(false),
            ..Default::default()
        };
        let args = CheckCommand {
            fix: true,
            unsafe_fixes: true,
            show_fixes: true,
            ..Default::default()
        };
        let config = CheckConfig::from_args(&args, &pyproject);
        assert!(config.fix);
        assert!(config.unsafe_fixes);
        assert!(config.show_fixes);
    }

    #[test]
    fn check_config_cli_no_overrides_pyproject() {
        let pyproject = PyprojectSettings {
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
        let config = CheckConfig::from_args(&args, &pyproject);
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
    fn summary_apply_to_disk_reports_fixed_and_remaining() {
        print_summary(2, 3, 0, 0, true, false, 0);
        assert!(logs_contain("Found 5 errors (3 fixed, 2 remaining)."));
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

    fn cli_default() -> CheckCommand {
        CheckCommand::default()
    }

    #[test]
    fn defaults_enable_invalid_attr_value() {
        let cli = cli_default();
        let pyproject = PyprojectSettings::default();
        let settings = resolve_settings(&cli, &pyproject);
        assert!(settings.is_enabled(Rule::InvalidAttrValue));
    }

    #[test]
    fn cli_ignore_disables_rule() {
        let cli = CheckCommand {
            ignore: Some(vec![RuleSelector::Rule(Rule::InvalidAttrValue)]),
            ..cli_default()
        };
        let pyproject = PyprojectSettings::default();
        let settings = resolve_settings(&cli, &pyproject);
        assert!(!settings.is_enabled(Rule::InvalidAttrValue));
    }

    #[test]
    fn cli_select_replaces_pyproject_select() {
        // pyproject says select = []; CLI says select = [ALL] — CLI wins.
        let cli = CheckCommand {
            select: Some(vec![RuleSelector::All]),
            ..cli_default()
        };
        let pyproject = PyprojectSettings {
            lint: Some(LintTomlSettings {
                select: Some(vec![]),
                ..Default::default()
            }),
            ..Default::default()
        };
        let settings = resolve_settings(&cli, &pyproject);
        assert!(settings.is_enabled(Rule::InvalidAttrValue));
    }

    #[test]
    fn pyproject_ignore_suppresses_then_cli_select_all_reenables() {
        // pyproject: ignore=[invalid-attr-value] -> suppressed at default ALL.
        let pyproject_only = PyprojectSettings {
            lint: Some(LintTomlSettings {
                ignore: Some(vec![RuleSelector::Rule(Rule::InvalidAttrValue)]),
                ..Default::default()
            }),
            ..Default::default()
        };
        let cli_only = cli_default();
        let settings = resolve_settings(&cli_only, &pyproject_only);
        assert!(!settings.is_enabled(Rule::InvalidAttrValue));

        // CLI `select=[ALL]` *replaces*: the new running set is recomputed
        // entirely from the CLI layer's selectors, discarding pyproject's
        // ignore. Matches ruff's `RuleSelection`-replacement semantics.
        let cli_with_select = CheckCommand {
            select: Some(vec![RuleSelector::All]),
            ..cli_default()
        };
        let settings = resolve_settings(&cli_with_select, &pyproject_only);
        assert!(settings.is_enabled(Rule::InvalidAttrValue));
    }

    #[test]
    fn cli_exact_select_beats_pyproject_category_ignore() {
        // pyproject: ignore=[correctness] (category level)
        // CLI: select=[invalid-attr-value] (rule level)
        // Rule-level specificity beats category-level: rule stays enabled.
        let pyproject = PyprojectSettings {
            lint: Some(LintTomlSettings {
                ignore: Some(vec![RuleSelector::Category(RuleCategory::Correctness)]),
                ..Default::default()
            }),
            ..Default::default()
        };
        let cli = CheckCommand {
            select: Some(vec![RuleSelector::Rule(Rule::InvalidAttrValue)]),
            ..cli_default()
        };
        let settings = resolve_settings(&cli, &pyproject);
        assert!(settings.is_enabled(Rule::InvalidAttrValue));
    }

    #[test]
    fn cli_extend_select_extends_pyproject() {
        // pyproject select = [] (nothing enabled). CLI extend-select=[correctness].
        let cli = CheckCommand {
            extend_select: Some(vec![RuleSelector::Category(RuleCategory::Correctness)]),
            ..cli_default()
        };
        let pyproject = PyprojectSettings {
            lint: Some(LintTomlSettings {
                select: Some(vec![]),
                ..Default::default()
            }),
            ..Default::default()
        };
        let settings = resolve_settings(&cli, &pyproject);
        assert!(settings.is_enabled(Rule::InvalidAttrValue));
    }

    #[test]
    fn preview_flag_precedence() {
        let cli_preview = CheckCommand {
            preview: true,
            ..cli_default()
        };
        let pyproject = PyprojectSettings {
            lint: Some(LintTomlSettings {
                preview: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            resolve_preview(&cli_preview, &pyproject),
            PreviewMode::Enabled
        );

        let cli_no_preview = CheckCommand {
            no_preview: true,
            ..cli_default()
        };
        let pyproject_preview = PyprojectSettings {
            lint: Some(LintTomlSettings {
                preview: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            resolve_preview(&cli_no_preview, &pyproject_preview),
            PreviewMode::Disabled
        );

        // Fallback to pyproject.
        assert_eq!(
            resolve_preview(&cli_default(), &pyproject_preview),
            PreviewMode::Enabled
        );

        // Fallback to disabled.
        assert_eq!(
            resolve_preview(&cli_default(), &PyprojectSettings::default()),
            PreviewMode::Disabled
        );
    }
}

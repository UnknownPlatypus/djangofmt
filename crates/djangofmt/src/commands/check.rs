use djangofmt_lint::{
    FileDiagnostics, PreviewMode, RuleSelection, RuleSelector, Settings, check_ast,
};
use markup_fmt::FormatError;
use markup_fmt::parser::Parser;
use miette::NamedSource;
use rayon::iter::Either::{Left, Right};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::fs;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error};

use crate::ExitStatus;
use crate::args::{CheckCommand, Profile};
use crate::error::{CommandError, ParseError, Result};
use crate::pyproject::{LintTomlSettings, PyprojectSettings};
use crate::resolver::resolve_bool_arg;

/// Check the given source code for linting errors.
pub fn check(args: &CheckCommand) -> Result<ExitStatus> {
    let resolved = super::resolve_command(&args.files, args.profile, &args.file_selection)?;
    let settings = resolve_settings(args, &resolved.pyproject);

    let start = Instant::now();
    let (file_diagnostics, mut parse_errors): (Vec<_>, Vec<_>) = resolved
        .files
        .par_iter()
        .map(|path| check_path(path, resolved.profile, &settings))
        .partition_map(|result| match result {
            Ok(diags) => Left(diags),
            Err(err) => Right(err),
        });

    let duration = start.elapsed();
    debug!("Checked {} files in {:.2?}", resolved.files.len(), duration);

    // Report on any parsing errors.
    parse_errors.sort_unstable_by(|a, b| a.path().cmp(&b.path()));
    let error_count = parse_errors.len();
    for error in parse_errors {
        error!("{:?}", miette::Report::new(*error));
    }
    if error_count > 0 {
        error!("Couldn't check {} files!", error_count);
    }

    // Filter out files with no diagnostics and count total
    let file_diagnostics: Vec<_> = file_diagnostics
        .into_iter()
        .filter(|fd| !fd.is_empty())
        .collect();
    let total_diagnostics: usize = file_diagnostics.iter().map(FileDiagnostics::len).sum();

    if total_diagnostics == 0 && error_count == 0 {
        return Ok(ExitStatus::Success);
    }

    // Report diagnostics per file
    for file_diag in file_diagnostics {
        error!("{:?}", miette::Report::new(file_diag));
    }

    Ok(ExitStatus::Failure)
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
#[tracing::instrument(level = "debug", skip_all, fields(path = %path.display()))]
fn check_path(
    path: &Path,
    profile: Option<Profile>,
    settings: &Settings,
) -> std::result::Result<FileDiagnostics, Box<CommandError>> {
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

    let diagnostics = check_ast(&source, &ast, settings);

    if diagnostics.is_empty() {
        return Ok(FileDiagnostics::empty());
    }
    Ok(FileDiagnostics::new(
        NamedSource::new(path.to_string_lossy(), source),
        diagnostics,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use djangofmt_lint::{Rule, RuleCategory};

    fn cli_default() -> CheckCommand {
        CheckCommand {
            files: vec![],
            profile: None,
            file_selection: crate::args::FileSelectionArgs::default(),
            select: None,
            ignore: None,
            extend_select: None,
            extend_ignore: None,
            preview: false,
            no_preview: false,
        }
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

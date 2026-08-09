//! Option resolution: merging CLI arguments, `pyproject.toml` settings and defaults.
//!
//! Precedence is always CLI > pyproject > default; [`resolve_profile`] extends
//! it with file-extension inference. Every command resolves through here so the
//! file and stdin paths cannot disagree.

use std::path::Path;

use djangofmt_lint::RuleSelection;

use crate::args::{Profile, RuleSelectionArgs};
use crate::pyproject::LintSettings;

/// Collapse a `--flag` / `--no-flag` pair into an optional bool.
#[must_use]
pub fn resolve_bool_arg(yes: bool, no: bool) -> Option<bool> {
    match (yes, no) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        (false, false) => None,
        (..) => unreachable!("Clap should make this impossible"),
    }
}

/// Resolve the template profile for a file.
///
/// Precedence: CLI > pyproject > file extension > default (Django).
#[must_use]
pub fn resolve_profile(
    cli: Option<Profile>,
    pyproject: Option<Profile>,
    path: Option<&Path>,
) -> Profile {
    cli.or(pyproject)
        .or_else(|| path.and_then(Profile::from_path))
        .unwrap_or_default()
}

/// Merge CLI rule-selection flags with `[tool.djangofmt.lint]` into a [`RuleSelection`].
#[must_use]
pub fn resolve_rule_selection(
    cli: &RuleSelectionArgs,
    lint: Option<&LintSettings>,
) -> RuleSelection {
    let select = cli
        .select
        .clone()
        .or_else(|| lint.and_then(|l| l.select.clone()));
    let ignore = cli
        .ignore
        .clone()
        .or_else(|| lint.and_then(|l| l.ignore.clone()))
        .unwrap_or_default();
    let preview = resolve_bool_arg(cli.preview, cli.no_preview)
        .or_else(|| lint.and_then(|l| l.preview))
        .unwrap_or(false);

    RuleSelection {
        select,
        ignore,
        preview,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_precedence() {
        let jinja_path = Some(Path::new("t.jinja"));
        // CLI beats everything.
        assert_eq!(
            resolve_profile(Some(Profile::Django), Some(Profile::Jinja), jinja_path),
            Profile::Django
        );
        // pyproject beats the file extension.
        assert_eq!(
            resolve_profile(None, Some(Profile::Django), jinja_path),
            Profile::Django
        );
        // The extension is used when nothing is configured.
        assert_eq!(resolve_profile(None, None, jinja_path), Profile::Jinja);
        // Django is the default.
        assert_eq!(resolve_profile(None, None, None), Profile::Django);
    }
}

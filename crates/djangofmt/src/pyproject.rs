use djangofmt_lint::RuleSelector;
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tracing::{debug, warn};

use crate::args::Profile;
use crate::line_width::{IndentWidth, LineLength, SelfClosing};

/// Serde-only struct for deserializing `[tool.djangofmt]` from `pyproject.toml`.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PyprojectSettings {
    pub line_length: Option<LineLength>,
    pub indent_width: Option<IndentWidth>,
    pub profile: Option<Profile>,
    pub custom_blocks: Option<Vec<String>>,
    pub html_void_self_closing: Option<SelfClosing>,
    pub preserve_unquoted_attrs: Option<bool>,
    pub exclude: Option<Vec<String>>,
    pub extend_exclude: Option<Vec<String>>,
    pub include: Option<Vec<String>>,
    pub extend_include: Option<Vec<String>>,
    pub respect_gitignore: Option<bool>,
    pub force_exclude: Option<bool>,
    pub fix: Option<bool>,
    pub unsafe_fixes: Option<bool>,
    pub show_fixes: Option<bool>,
    /// Optional `[tool.djangofmt.lint]` subtable.
    ///
    /// Deserialized leniently (see `deserialize_lenient_lint`): a key with an
    /// invalid value or an unknown name is skipped with a warning, keeping the
    /// other lint keys and the surrounding formatting options.
    #[serde(default, deserialize_with = "deserialize_lenient_lint")]
    pub lint: Option<LintTomlSettings>,
}

/// The parsed `[tool.djangofmt.lint]` subtable, built key-by-key by
/// [`deserialize_lenient_lint`].
///
/// `select` is `Option<Vec<RuleSelector>>` (not `Vec<RuleSelector>`) so that
/// a missing key (`None`) is distinguishable from an explicit empty list
/// (`Some(vec![])`). This is load-bearing: at the resolver, `Some(select)`
/// *replaces* the carried select set, whereas `None` extends only via
/// `extend_select` / `extend_ignore` / `ignore`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LintTomlSettings {
    pub select: Option<Vec<RuleSelector>>,
    pub ignore: Option<Vec<RuleSelector>>,
    pub extend_select: Option<Vec<RuleSelector>>,
    pub extend_ignore: Option<Vec<RuleSelector>>,
    pub preview: Option<bool>,
}

/// Deserialize the optional `[tool.djangofmt.lint]` subtable leniently.
///
/// The table is buffered into a [`toml::Value`] and each key is converted
/// independently: a key with an invalid value (e.g. a typo'd selector) or an
/// unrecognised name is skipped with a warning, while the other lint keys —
/// and the surrounding formatting options like `line-length` — are preserved.
/// A single bad key can never discard unrelated configuration.
fn deserialize_lenient_lint<'de, D>(deserializer: D) -> Result<Option<LintTomlSettings>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = toml::Value::deserialize(deserializer)?;
    let toml::Value::Table(table) = value else {
        warn!("Ignoring `[tool.djangofmt.lint]`: expected a table");
        return Ok(None);
    };

    let mut lint = LintTomlSettings::default();
    for (key, value) in table {
        let result = match key.as_str() {
            "select" => take_field(&mut lint.select, value),
            "ignore" => take_field(&mut lint.ignore, value),
            "extend-select" => take_field(&mut lint.extend_select, value),
            "extend-ignore" => take_field(&mut lint.extend_ignore, value),
            "preview" => take_field(&mut lint.preview, value),
            _ => {
                warn!("Ignoring unknown `[tool.djangofmt.lint]` key `{key}`");
                continue;
            }
        };
        if let Err(err) = result {
            warn!("Ignoring invalid `[tool.djangofmt.lint] {key}`: {err}");
        }
    }
    Ok(Some(lint))
}

/// Parse `value` into `slot`, leaving `slot` untouched (and returning the
/// error) when deserialization fails.
fn take_field<T: serde::de::DeserializeOwned>(
    slot: &mut Option<T>,
    value: toml::Value,
) -> Result<(), toml::de::Error> {
    *slot = Some(value.try_into()?);
    Ok(())
}

#[derive(Deserialize, Debug)]
struct PyProject {
    tool: Option<Tool>,
}

#[derive(Deserialize, Debug)]
struct Tool {
    #[serde(default)]
    djangofmt: Option<PyprojectSettings>,
}

/// Loads `Options` from a given `pyproject.toml` file
fn load_options_from_pyproject_toml(content: &str) -> PyprojectSettings {
    match toml::from_str::<PyProject>(content) {
        Ok(pyproject) => pyproject.tool.and_then(|t| t.djangofmt).unwrap_or_default(),
        Err(err) => {
            warn!("Failed to parse pyproject.toml: {err}");
            PyprojectSettings::default()
        }
    }
}

/// Finds the `pyproject.toml` settings file by traversing directories upward from the given path
fn find_pyproject_toml<P: AsRef<Path>>(start_path: P) -> Option<PathBuf> {
    for directory in start_path.as_ref().ancestors() {
        let pyproject_toml = directory.join("pyproject.toml");
        if pyproject_toml.is_file() {
            return Some(pyproject_toml);
        }
    }
    None
}

/// Load `pyproject.toml` settings rooted at the current working directory,
/// falling back to defaults if the cwd can't be determined.
#[must_use]
pub fn load_pyproject_from_cwd() -> PyprojectSettings {
    load_options(crate::fs::get_cwd())
}

/// Loads user configured options from the nearest `pyproject.toml` file from the given path
pub fn load_options<P: AsRef<Path>>(start_path: P) -> PyprojectSettings {
    let Some(pyproject_path) = find_pyproject_toml(start_path.as_ref()) else {
        debug!(
            "No pyproject.toml found starting search from: {}",
            start_path.as_ref().display()
        );
        return PyprojectSettings::default();
    };
    debug!(
        "Loading options from pyproject.toml at: {}",
        pyproject_path.display()
    );

    let content = match fs::read_to_string(&pyproject_path) {
        Ok(c) => c,
        Err(err) => {
            warn!("Failed to read {}: {}", pyproject_path.display(), err);
            return PyprojectSettings::default();
        }
    };
    load_options_from_pyproject_toml(&content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use tempfile::tempdir;

    #[test]
    fn test_find_pyproject_toml_should_return_none() {
        let temp_dir = tempdir().unwrap();
        assert_eq!(find_pyproject_toml(temp_dir), None);
    }

    #[test]
    fn test_find_pyproject_toml_in_current_dir() {
        let temp_dir = tempdir().unwrap();
        let pyproject_path = temp_dir.path().join("pyproject.toml");
        fs::write(&pyproject_path, "").unwrap();
        assert_eq!(find_pyproject_toml(temp_dir), Some(pyproject_path));
    }

    #[test]
    fn test_find_pyproject_toml_in_parent_dir() {
        let parent_dir = tempdir().unwrap();
        let pyproject_path = parent_dir.path().join("pyproject.toml");
        fs::write(&pyproject_path, "").unwrap();
        fs::create_dir(parent_dir.path().join("child_dir")).unwrap();
        let child_dir = parent_dir.path().join("child_dir");
        assert_eq!(find_pyproject_toml(child_dir), Some(pyproject_path));
    }

    #[test]
    fn test_load_options_from_pyproject_toml() {
        let temp_dir = tempdir().unwrap();
        let pyproject_path = temp_dir.path().join("pyproject.toml");
        let pyproject_content = r"
            [tool.djangofmt]
            line-length=200
            indent-width=4
            custom-blocks=['foo', 'bar']
            profile='django'
            html-void-self-closing='always'
            ";

        fs::write(&pyproject_path, pyproject_content).unwrap();
        let result = load_options(&pyproject_path);
        assert_eq!(
            result,
            PyprojectSettings {
                line_length: Some(LineLength::try_from(200).unwrap()),
                indent_width: Some(IndentWidth::try_from(4).unwrap()),
                custom_blocks: Some(vec!["foo".to_string(), "bar".to_string()]),
                profile: Some(Profile::Django),
                html_void_self_closing: Some(SelfClosing::Always),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_load_options_from_incomplete_pyproject_toml() {
        let temp_dir = tempdir().unwrap();
        let pyproject_path = temp_dir.path().join("pyproject.toml");
        let pyproject_content = r"
            [tool.djangofmt]
            line-length=200
            ";

        fs::write(&pyproject_path, pyproject_content).unwrap();
        let result = load_options(&pyproject_path);
        assert_eq!(
            result,
            PyprojectSettings {
                line_length: Some(LineLength::try_from(200).unwrap()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_load_options_returns_default_when_no_pyproject_toml() {
        let temp_dir = tempdir().unwrap();
        let result = load_options(temp_dir.path());
        assert_eq!(result, PyprojectSettings::default());
    }

    #[test]
    fn test_load_options_returns_default_when_empty_pyproject_toml() {
        let temp_dir = tempdir().unwrap();
        let pyproject_path = temp_dir.path().join("pyproject.toml");
        fs::write(&pyproject_path, "").unwrap();
        let result = load_options(&pyproject_path);
        assert_eq!(result, PyprojectSettings::default());
    }

    #[rstest]
    #[case("[tool.djangofmt]\nunknown-option = 100")]
    #[case("[tool.djangofmt]\nline-length = 0")]
    #[case("[tool.djangofmt]\nline-length = 321")]
    #[case("[tool.djangofmt]\nindent-width = 0")]
    #[case("[tool.djangofmt]\nindent-width = 17")]
    fn test_load_options_returns_default_on_invalid_toml(#[case] content: &str) {
        assert_eq!(
            load_options_from_pyproject_toml(content),
            PyprojectSettings::default()
        );
    }

    #[test]
    fn test_load_options_with_file_selection_fields() {
        let content = r#"
        [tool.djangofmt]
        line-length = 120
        exclude = [".git", ".venv"]
        extend-exclude = ["vendor"]
        include = ["*.html"]
        extend-include = ["*.djhtml"]
        respect-gitignore = false
    "#;
        let result = load_options_from_pyproject_toml(content);
        assert_eq!(
            result,
            PyprojectSettings {
                line_length: Some(LineLength::try_from(120).unwrap()),
                exclude: Some(vec![".git".to_string(), ".venv".to_string()]),
                extend_exclude: Some(vec!["vendor".to_string()]),
                include: Some(vec!["*.html".to_string()]),
                extend_include: Some(vec!["*.djhtml".to_string()]),
                respect_gitignore: Some(false),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_load_options_with_only_new_fields_defaults_rest() {
        let content = r#"
        [tool.djangofmt]
        extend-exclude = ["build"]
    "#;
        let result = load_options_from_pyproject_toml(content);
        assert_eq!(
            result,
            PyprojectSettings {
                extend_exclude: Some(vec!["build".to_string()]),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_load_preserve_unquoted_attrs() {
        let content = r"
[tool.djangofmt]
preserve-unquoted-attrs = true
";
        let result = load_options_from_pyproject_toml(content);
        assert_eq!(
            result,
            PyprojectSettings {
                preserve_unquoted_attrs: Some(true),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_load_fix_flags() {
        let content = r"
[tool.djangofmt]
fix = true
unsafe-fixes = true
show-fixes = true
";
        let result = load_options_from_pyproject_toml(content);
        assert_eq!(
            result,
            PyprojectSettings {
                fix: Some(true),
                unsafe_fixes: Some(true),
                show_fixes: Some(true),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_load_options_existing_fields_still_work() {
        let content = r#"
        [tool.djangofmt]
        line-length = 80
        indent-width = 2
        profile = "jinja"
        custom-blocks = ["cache"]
    "#;
        let result = load_options_from_pyproject_toml(content);
        assert_eq!(
            result,
            PyprojectSettings {
                line_length: Some(LineLength::try_from(80).unwrap()),
                indent_width: Some(IndentWidth::try_from(2).unwrap()),
                profile: Some(Profile::Jinja),
                custom_blocks: Some(vec!["cache".to_string()]),
                ..Default::default()
            }
        );
    }

    // ── [tool.djangofmt.lint] ──────────────────────────────────────────

    #[test]
    fn test_load_options_with_full_lint_table() {
        use djangofmt_lint::{Rule, RuleCategory, RuleSelector};
        let content = r#"
        [tool.djangofmt.lint]
        select = ["ALL"]
        ignore = ["invalid-attr-value"]
        extend-select = ["correctness"]
        extend-ignore = ["correctness"]
        preview = true
    "#;
        let result = load_options_from_pyproject_toml(content);
        assert_eq!(
            result,
            PyprojectSettings {
                lint: Some(LintTomlSettings {
                    select: Some(vec![RuleSelector::All]),
                    ignore: Some(vec![RuleSelector::Rule(Rule::InvalidAttrValue)]),
                    extend_select: Some(vec![RuleSelector::Category(RuleCategory::Correctness)]),
                    extend_ignore: Some(vec![RuleSelector::Category(RuleCategory::Correctness)]),
                    preview: Some(true),
                }),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_load_options_with_partial_lint_table() {
        use djangofmt_lint::{Rule, RuleSelector};
        let content = r#"
        [tool.djangofmt.lint]
        ignore = ["invalid-attr-value"]
    "#;
        let result = load_options_from_pyproject_toml(content);
        assert_eq!(
            result,
            PyprojectSettings {
                lint: Some(LintTomlSettings {
                    ignore: Some(vec![RuleSelector::Rule(Rule::InvalidAttrValue)]),
                    ..Default::default()
                }),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_load_options_unknown_key_in_lint_table_is_skipped() {
        use djangofmt_lint::{Rule, RuleSelector};
        // An unknown key is skipped with a warning; sibling keys are kept.
        let content = r#"
        [tool.djangofmt.lint]
        bogus = "value"
        ignore = ["invalid-attr-value"]
    "#;
        let result = load_options_from_pyproject_toml(content);
        assert_eq!(
            result.lint,
            Some(LintTomlSettings {
                ignore: Some(vec![RuleSelector::Rule(Rule::InvalidAttrValue)]),
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_load_options_with_empty_select_list() {
        // `select = []` must round-trip as `Some(vec![])` — distinct from
        // `select` being missing entirely (which is `None`).
        let content = r"
        [tool.djangofmt.lint]
        select = []
    ";
        let result = load_options_from_pyproject_toml(content);
        assert_eq!(
            result.lint,
            Some(LintTomlSettings {
                select: Some(vec![]),
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_load_options_with_missing_select_is_none() {
        // No `select` key — `lint.select` must be `None`, not `Some(vec![])`.
        let content = r"
        [tool.djangofmt.lint]
        preview = false
    ";
        let result = load_options_from_pyproject_toml(content);
        assert_eq!(
            result.lint,
            Some(LintTomlSettings {
                select: None,
                preview: Some(false),
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_load_options_skips_only_invalid_selector_key() {
        use djangofmt_lint::{Rule, RuleSelector};
        // A key whose value has a bad selector is skipped (left as `None`),
        // while valid sibling keys survive.
        let content = r#"
        [tool.djangofmt.lint]
        select = ["definitely-not-a-rule"]
        ignore = ["invalid-attr-value"]
        preview = true
    "#;
        let result = load_options_from_pyproject_toml(content);
        assert_eq!(
            result.lint,
            Some(LintTomlSettings {
                select: None,
                ignore: Some(vec![RuleSelector::Rule(Rule::InvalidAttrValue)]),
                preview: Some(true),
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_invalid_lint_table_preserves_other_settings() {
        // A bad selector in `[tool.djangofmt.lint]` must not discard unrelated
        // formatting options like `line-length`.
        let content = r#"
        [tool.djangofmt]
        line-length = 100
        [tool.djangofmt.lint]
        select = ["definitely-not-a-rule"]
    "#;
        let result = load_options_from_pyproject_toml(content);
        assert_eq!(result.line_length, Some(LineLength::try_from(100).unwrap()));
        // The bad `select` is skipped, leaving an otherwise-empty lint table.
        assert_eq!(result.lint, Some(LintTomlSettings::default()));
    }

    #[test]
    fn test_unknown_key_in_lint_table_preserves_other_settings() {
        // An unknown key in the lint table is skipped and scoped to the lint
        // table only, leaving formatting options untouched.
        let content = r"
        [tool.djangofmt]
        line-length = 100
        [tool.djangofmt.lint]
        not-a-real-key = true
    ";
        let result = load_options_from_pyproject_toml(content);
        assert_eq!(result.line_length, Some(LineLength::try_from(100).unwrap()));
        assert_eq!(result.lint, Some(LintTomlSettings::default()));
    }
}

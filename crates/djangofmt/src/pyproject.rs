use djangofmt_lint::RuleSelector;
use serde::Deserialize;
use std::{fs, path::Path};
use tracing::debug;

use crate::args::Profile;
use crate::error::{Error, Result};
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
    pub lint: Option<LintSettings>,
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct LintSettings {
    pub select: Option<Vec<RuleSelector>>,
    pub ignore: Option<Vec<RuleSelector>>,
    pub preview: Option<bool>,
    pub fix: Option<bool>,
    pub unsafe_fixes: Option<bool>,
    pub show_fixes: Option<bool>,
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
fn load_options_from_pyproject_toml(content: &str) -> Result<PyprojectSettings> {
    let pyproject = toml::from_str::<PyProject>(content)
        .map_err(|err| Error::Resolve(format!("Failed to parse pyproject.toml: {err}")))?;
    Ok(pyproject.tool.and_then(|t| t.djangofmt).unwrap_or_default())
}

/// Load `pyproject.toml` settings rooted at the current working directory,
/// falling back to defaults if the cwd can't be determined.
pub fn load_pyproject_from_cwd() -> Result<PyprojectSettings> {
    load_options(crate::fs::get_cwd())
}

/// Loads user configured options from the nearest `pyproject.toml` file from the given path
pub fn load_options<P: AsRef<Path>>(start_path: P) -> Result<PyprojectSettings> {
    let Some(pyproject_path) =
        crate::fs::find_nearest_ancestor_file(start_path.as_ref(), "pyproject.toml")
    else {
        debug!(
            "No pyproject.toml found starting search from: {}",
            start_path.as_ref().display()
        );
        return Ok(PyprojectSettings::default());
    };
    debug!(
        "Loading options from pyproject.toml at: {}",
        pyproject_path.display()
    );

    let content = fs::read_to_string(&pyproject_path).map_err(|err| {
        Error::Resolve(format!(
            "Failed to read {}: {err}",
            pyproject_path.display()
        ))
    })?;
    load_options_from_pyproject_toml(&content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Project;
    use rstest::rstest;

    #[test]
    fn test_load_options_from_pyproject_toml() {
        let project = Project::new().file(
            "pyproject.toml",
            r"
            [tool.djangofmt]
            line-length=200
            indent-width=4
            custom-blocks=['foo', 'bar']
            profile='django'
            html-void-self-closing='always'
            ",
        );
        let result = load_options(project.join("pyproject.toml")).unwrap();
        assert_eq!(
            result,
            PyprojectSettings {
                line_length: Some(LineLength::try_from(200u16).unwrap()),
                indent_width: Some(IndentWidth::try_from(4u8).unwrap()),
                custom_blocks: Some(vec!["foo".to_string(), "bar".to_string()]),
                profile: Some(Profile::Django),
                html_void_self_closing: Some(SelfClosing::Always),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_load_options_from_incomplete_pyproject_toml() {
        let project = Project::new().file(
            "pyproject.toml",
            r"
            [tool.djangofmt]
            line-length=200
            ",
        );
        let result = load_options(project.join("pyproject.toml")).unwrap();
        assert_eq!(
            result,
            PyprojectSettings {
                line_length: Some(LineLength::try_from(200u16).unwrap()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_load_options_returns_default_when_no_pyproject_toml() {
        let project = Project::new();
        let result = load_options(project.path()).unwrap();
        assert_eq!(result, PyprojectSettings::default());
    }

    #[test]
    fn test_load_options_returns_default_when_empty_pyproject_toml() {
        let project = Project::new().file("pyproject.toml", "");
        let result = load_options(project.join("pyproject.toml")).unwrap();
        assert_eq!(result, PyprojectSettings::default());
    }

    #[rstest]
    #[case("[tool.djangofmt]\nunknown-option = 100")]
    #[case("[tool.djangofmt]\nline-length = 0")]
    #[case("[tool.djangofmt]\nline-length = 321")]
    #[case("[tool.djangofmt]\nindent-width = 0")]
    #[case("[tool.djangofmt]\nindent-width = 17")]
    #[case("[tool.djangofmt.lint]\nselect = [\"not-a-real-rule\"]")]
    fn test_load_options_errors_on_invalid_toml(#[case] content: &str) {
        // Invalid config (including unknown lint selectors) must fail fast rather
        // than silently falling back to defaults.
        assert!(load_options_from_pyproject_toml(content).is_err());
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
        let result = load_options_from_pyproject_toml(content).unwrap();
        assert_eq!(
            result,
            PyprojectSettings {
                line_length: Some(LineLength::try_from(120u16).unwrap()),
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
        let result = load_options_from_pyproject_toml(content).unwrap();
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
        let result = load_options_from_pyproject_toml(content).unwrap();
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
[tool.djangofmt.lint]
fix = true
unsafe-fixes = true
show-fixes = true
";
        let result = load_options_from_pyproject_toml(content).unwrap();
        assert_eq!(
            result,
            PyprojectSettings {
                lint: Some(LintSettings {
                    fix: Some(true),
                    unsafe_fixes: Some(true),
                    show_fixes: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_load_lint_select_ignore_preview() {
        use djangofmt_lint::{Rule, RuleCategory};

        let content = r#"
[tool.djangofmt.lint]
select = ["category:all"]
ignore = ["category:style", "missing-img-alt"]
preview = true
"#;
        let result = load_options_from_pyproject_toml(content).unwrap();
        assert_eq!(
            result,
            PyprojectSettings {
                lint: Some(LintSettings {
                    select: Some(vec![RuleSelector::All]),
                    ignore: Some(vec![
                        RuleSelector::Category(RuleCategory::Style),
                        RuleSelector::Rule(Rule::MissingImgAlt)
                    ]),
                    preview: Some(true),
                    ..Default::default()
                }),
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
        let result = load_options_from_pyproject_toml(content).unwrap();
        assert_eq!(
            result,
            PyprojectSettings {
                line_length: Some(LineLength::try_from(80u16).unwrap()),
                indent_width: Some(IndentWidth::try_from(2u8).unwrap()),
                profile: Some(Profile::Jinja),
                custom_blocks: Some(vec!["cache".to_string()]),
                ..Default::default()
            }
        );
    }
}

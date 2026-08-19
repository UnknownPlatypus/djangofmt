use djangofmt_lint::RuleSelector;
use djangofmt_lint::settings::unsorted_tailwind_classes;
use djangofmt_macros::OptionsMetadata;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use tracing::debug;

use crate::args::{OutputFormat, Profile};
use crate::error::{Error, Result};
use crate::line_width::{IndentWidth, LineLength, SelfClosing};

/// Options shared by the `format` and `check` commands.
#[derive(Debug, Default, Deserialize, PartialEq, Eq, OptionsMetadata)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PyprojectSettings {
    /// The line length the formatter should fit code into when possible.
    #[option(default = "120", value_type = "int", example = "line-length = 88")]
    pub line_length: Option<LineLength>,

    /// The number of spaces per indentation level.
    #[option(default = "4", value_type = "int", example = "indent-width = 2")]
    pub indent_width: Option<IndentWidth>,

    /// The template language to parse. Defaults to the file extension when unset
    /// (`.html` is Django, `.jinja`/`.jinja2`/`.j2` are Jinja).
    #[option(
        default = r#""django""#,
        value_type = r#""django" | "jinja""#,
        example = r#"profile = "jinja""#
    )]
    pub profile: Option<Profile>,

    /// Names of custom block tags to treat as paired blocks, so their content is indented
    /// instead of left untouched.
    #[option(
        default = "[]",
        value_type = "list[str]",
        example = r#"custom-blocks = ["cache", "spaceless"]"#
    )]
    pub custom_blocks: Option<Vec<String>>,

    /// Whether void HTML elements are written self-closing (`<br />`) or not (`<br>`).
    /// `unchanged` keeps whatever the source uses.
    #[option(
        default = r#""never""#,
        value_type = r#""never" | "always" | "unchanged""#,
        example = r#"html-void-self-closing = "always""#
    )]
    pub html_void_self_closing: Option<SelfClosing>,

    /// Whether to leave unquoted attribute values (e.g. `prop=True`) as-is instead of quoting
    /// them. Useful for template syntaxes that assign non-string values through attributes.
    #[option(
        default = "false",
        value_type = "bool",
        example = "preserve-unquoted-attrs = true"
    )]
    pub preserve_unquoted_attrs: Option<bool>,

    /// File and directory patterns to exclude from discovery, replacing the default excludes.
    #[option(
        default = r#"[".bzr", ".direnv", ".eggs", ".git", ".git-rewrite", ".hg", ".mypy_cache", ".nox", ".pants.d", ".pytype", ".ruff_cache", ".svn", ".tox", ".venv", "__pypackages__", "_build", "buck-out", "dist", "node_modules", "venv"]"#,
        value_type = "list[str]",
        example = r#"exclude = ["generated"]"#
    )]
    pub exclude: Option<Vec<String>>,

    /// File and directory patterns to exclude in addition to the default excludes.
    #[option(
        default = "[]",
        value_type = "list[str]",
        example = r#"extend-exclude = ["templates/vendor"]"#
    )]
    pub extend_exclude: Option<Vec<String>>,

    /// File patterns to format and lint, replacing the default includes.
    #[option(
        default = r#"["*.html", "*.jinja", "*.jinja2", "*.j2"]"#,
        value_type = "list[str]",
        example = r#"include = ["*.html"]"#
    )]
    pub include: Option<Vec<String>>,

    /// File patterns to format and lint in addition to the default includes.
    #[option(
        default = "[]",
        value_type = "list[str]",
        example = r#"extend-include = ["*.djhtml"]"#
    )]
    pub extend_include: Option<Vec<String>>,

    /// Whether to skip files ignored by `.gitignore`, `.ignore` and friends when discovering files.
    #[option(
        default = "true",
        value_type = "bool",
        example = "respect-gitignore = false"
    )]
    pub respect_gitignore: Option<bool>,

    /// Whether to apply `exclude` patterns to files passed on the command line too. Useful when
    /// running under pre-commit, which passes every changed file explicitly.
    #[option(
        default = "false",
        value_type = "bool",
        example = "force-exclude = true"
    )]
    pub force_exclude: Option<bool>,

    #[option_group]
    pub lint: Option<LintSettings>,
}

/// Options for the `check` command.
#[derive(Debug, Default, Deserialize, PartialEq, Eq, OptionsMetadata)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct LintSettings {
    /// Rules and categories to enable, e.g. `category:all`, `category:correctness` or a rule name.
    #[option(
        default = r#"["category:all"]"#,
        value_type = "list[RuleSelector]",
        example = r#"select = ["category:correctness", "use-https"]"#
    )]
    pub select: Option<Vec<RuleSelector>>,

    /// Rules and categories to disable. A more specific selector always wins, regardless of order.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"ignore = ["category:style"]"#
    )]
    pub ignore: Option<Vec<RuleSelector>>,

    /// Whether to enable rules that are still in preview.
    #[option(default = "false", value_type = "bool", example = "preview = true")]
    pub preview: Option<bool>,

    /// Whether to apply safe fixes automatically.
    #[option(default = "false", value_type = "bool", example = "fix = true")]
    pub fix: Option<bool>,

    /// Whether to include unsafe fixes when applying with `fix`. Without `fix`, diagnostics from
    /// unsafe fixes are still reported as fixable.
    #[option(
        default = "false",
        value_type = "bool",
        example = "unsafe-fixes = true"
    )]
    pub unsafe_fixes: Option<bool>,

    /// Whether to list per-rule fix counts after applying fixes.
    #[option(default = "false", value_type = "bool", example = "show-fixes = true")]
    pub show_fixes: Option<bool>,

    /// How diagnostics are rendered.
    #[option(
        default = r#""full""#,
        value_type = r#""full" | "concise""#,
        example = r#"output-format = "concise""#
    )]
    pub output_format: Option<OutputFormat>,

    /// Rules to disable for files matching a glob pattern, relative to the `pyproject.toml`
    /// directory.
    #[option(
        default = "{}",
        value_type = "dict[str, list[RuleSelector]]",
        scope = "per-file-ignores",
        example = r#""templates/admin/*.html" = ["missing-img-alt"]"#
    )]
    pub per_file_ignores: Option<BTreeMap<String, Vec<RuleSelector>>>,

    #[option_group]
    pub unsorted_tailwind_classes: Option<UnsortedTailwindClassesOptions>,
}

/// Options for the [`unsorted-tailwind-classes`](rules/unsorted-tailwind-classes.md) rule.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq, OptionsMetadata)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct UnsortedTailwindClassesOptions {
    /// The Tailwind prefix your utilities are configured with. Without it, prefixed utilities are
    /// treated as unknown classes and left unsorted.
    #[option(
        default = "null",
        value_type = "str",
        example = r#"prefix = "tw-"  # Tailwind v3; use "tw:" for v4"#
    )]
    pub prefix: Option<String>,
}

impl UnsortedTailwindClassesOptions {
    #[must_use]
    pub fn into_settings(self) -> unsorted_tailwind_classes::Settings {
        unsorted_tailwind_classes::Settings {
            prefix: self.prefix,
        }
    }
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
pub fn load_pyproject_from_cwd() -> Result<(PyprojectSettings, PathBuf)> {
    load_options(crate::fs::get_cwd())
}

/// Loads user configured options from the nearest `pyproject.toml` file from the given path.
///
/// Also returns the directory that file was found in (the search start when there is none),
/// which anchors path-relative config such as `per-file-ignores`.
pub fn load_options<P: AsRef<Path>>(start_path: P) -> Result<(PyprojectSettings, PathBuf)> {
    let Some(pyproject_path) =
        crate::fs::find_nearest_ancestor_file(start_path.as_ref(), "pyproject.toml")
    else {
        debug!(
            "No pyproject.toml found starting search from: {}",
            start_path.as_ref().display()
        );
        return Ok((
            PyprojectSettings::default(),
            start_path.as_ref().to_path_buf(),
        ));
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
    let root = pyproject_path
        .parent()
        .map_or_else(|| start_path.as_ref().to_path_buf(), Path::to_path_buf);
    Ok((load_options_from_pyproject_toml(&content)?, root))
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
        let (result, _) = load_options(project.join("pyproject.toml")).unwrap();
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
        let (result, _) = load_options(project.join("pyproject.toml")).unwrap();
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
        let (result, _) = load_options(project.path()).unwrap();
        assert_eq!(result, PyprojectSettings::default());
    }

    #[test]
    fn test_load_options_returns_default_when_empty_pyproject_toml() {
        let project = Project::new().file("pyproject.toml", "");
        let (result, _) = load_options(project.join("pyproject.toml")).unwrap();
        assert_eq!(result, PyprojectSettings::default());
    }

    #[rstest]
    #[case("[tool.djangofmt]\nunknown-option = 100")]
    #[case("[tool.djangofmt]\nline-length = 0")]
    #[case("[tool.djangofmt]\nline-length = 321")]
    #[case("[tool.djangofmt]\nindent-width = 0")]
    #[case("[tool.djangofmt]\nindent-width = 17")]
    #[case("[tool.djangofmt.lint]\nselect = [\"not-a-real-rule\"]")]
    #[case("[tool.djangofmt.lint.unsorted-tailwind-classes]\nunknown-key = 1")]
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
    fn test_load_lint_unsorted_tailwind_classes_prefix() {
        let content = r#"
[tool.djangofmt.lint.unsorted-tailwind-classes]
prefix = "tw-"
"#;
        let result = load_options_from_pyproject_toml(content).unwrap();
        assert_eq!(
            result,
            PyprojectSettings {
                lint: Some(LintSettings {
                    unsorted_tailwind_classes: Some(UnsortedTailwindClassesOptions {
                        prefix: Some("tw-".to_string()),
                    }),
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
    fn test_load_lint_per_file_ignores() {
        use djangofmt_lint::Rule;

        let content = r#"
[tool.djangofmt.lint.per-file-ignores]
"templates/admin/*.html" = ["missing-img-alt"]
"#;
        let result = load_options_from_pyproject_toml(content).unwrap();
        let per_file = result.lint.unwrap().per_file_ignores.unwrap();
        assert_eq!(
            per_file.get("templates/admin/*.html"),
            Some(&vec![RuleSelector::Rule(Rule::MissingImgAlt)])
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

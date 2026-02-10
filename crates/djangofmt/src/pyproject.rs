use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tracing::warn;

use crate::args::Profile;
use crate::line_width::{IndentWidth, LineLength};

/// Serde-only struct for deserializing `[tool.djangofmt]` from `pyproject.toml`.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PyprojectSettings {
    pub line_length: Option<LineLength>,
    pub indent_width: Option<IndentWidth>,
    pub profile: Option<Profile>,
    pub custom_blocks: Option<Vec<String>>,
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

/// Loads user configured options from the nearest `pyproject.toml` file from the given path
pub fn load_options<P: AsRef<Path>>(start_path: P) -> PyprojectSettings {
    let Some(pyproject_path) = find_pyproject_toml(start_path.as_ref()) else {
        return PyprojectSettings::default();
    };
    let Ok(content) = fs::read_to_string(&pyproject_path) else {
        return PyprojectSettings::default();
    };
    load_options_from_pyproject_toml(&content)
}

#[cfg(test)]
mod tests {
    use super::*;
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
        fs::create_dir(parent_dir.path().join("child_dir")).ok();
        let child_dir = parent_dir.path().join("child_dir");
        assert_eq!(find_pyproject_toml(child_dir), Some(pyproject_path));
    }

    #[test]
    fn test_load_options_from_pyproject_toml() {
        let temp_dir = tempdir().unwrap();
        let pyproject_path = temp_dir.path().join("pyproject.toml");
        let pyproject_content = r"
            [tool.djangofmt]
            line_length=200
            indent_width=4
            custom_blocks=['foo', 'bar']
            profile='Django'
            ";

        fs::write(&pyproject_path, pyproject_content).unwrap();
        let result = load_options(&pyproject_path);
        assert_eq!(
            result,
            PyprojectSettings {
                line_length: Some(LineLength::try_from(200).unwrap()),
                indent_width: Some(IndentWidth::try_from(4).unwrap()),
                custom_blocks: Some(vec!["foo".to_string(), "bar".to_string()]),
                profile: Some(Profile::Django)
            }
        );
    }

    #[test]
    fn test_load_options_from_incomplete_pyproject_toml() {
        let temp_dir = tempdir().unwrap();
        let pyproject_path = temp_dir.path().join("pyproject.toml");
        let pyproject_content = r"
            [tool.djangofmt]
            line_length=200
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
        let pyproject_content = r"";
        fs::write(&pyproject_path, pyproject_content).unwrap();
        let result = load_options(&pyproject_path);
        assert_eq!(result, PyprojectSettings::default());
    }

    #[test]
    fn test_load_options_returns_default_on_unknown_field() {
        let content = r"
            [tool.djangofmt]
            unknown_option = 100
        ";
        assert_eq!(
            load_options_from_pyproject_toml(content),
            PyprojectSettings::default()
        );
    }

    #[test]
    fn test_load_options_returns_default_on_invalid_line_length() {
        let content = r"
            [tool.djangofmt]
            line_length = 0
        ";
        assert_eq!(
            load_options_from_pyproject_toml(content),
            PyprojectSettings::default()
        );

        let content = r"
            [tool.djangofmt]
            line_length = 321
        ";
        assert_eq!(
            load_options_from_pyproject_toml(content),
            PyprojectSettings::default()
        );
    }

    #[test]
    fn test_load_options_returns_default_on_invalid_indent_width() {
        let content = r"
            [tool.djangofmt]
            indent_width = 0
        ";
        assert_eq!(
            load_options_from_pyproject_toml(content),
            PyprojectSettings::default()
        );

        let content = r"
            [tool.djangofmt]
            indent_width = 17
        ";
        assert_eq!(
            load_options_from_pyproject_toml(content),
            PyprojectSettings::default()
        );
    }
}

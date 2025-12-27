use markup_fmt::Language;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Deserialize)]
struct PyProject {
    tool: Option<Tool>,
}

#[derive(Deserialize)]
struct Tool {
    #[serde(default)]
    djangofmt: Option<DjangoFmtOptions>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, clap::ValueEnum, Default)]
pub enum Profile {
    #[default]
    Django,
    Jinja,
}

impl From<&Profile> for Language {
    fn from(profile: &Profile) -> Self {
        match profile {
            Profile::Django => Self::Django,
            Profile::Jinja => Self::Jinja,
        }
    }
}

impl From<&str> for Profile {
    fn from(s: &str) -> Self {
        match s {
            "django" => Self::Django,
            "jinja" => Self::Jinja,
            _ => Self::default(),
        }
    }
}

impl FromStr for Profile {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct DjangoFmtOptions {
    pub line_length: usize,
    pub indent_width: usize,
    pub custom_blocks: Vec<String>,
    pub profile: Profile,
}

impl Default for DjangoFmtOptions {
    fn default() -> Self {
        Self {
            line_length: 120,
            indent_width: 4,
            custom_blocks: vec![],
            profile: Profile::default(),
        }
    }
}

impl DjangoFmtOptions {
    #[must_use]
    pub fn new(
        line_length: Option<usize>,
        indent_width: Option<usize>,
        custom_blocks: Option<Vec<String>>,
        profile: Option<Profile>,
    ) -> Self {
        let default = Self::default();
        Self {
            line_length: line_length.unwrap_or(default.line_length),
            indent_width: indent_width.unwrap_or(default.indent_width),
            custom_blocks: custom_blocks.unwrap_or(default.custom_blocks),
            profile: profile.unwrap_or(default.profile),
        }
    }
}

/// Loads `FormatterOptions` from a given `pyproject.toml` file
fn load_options_from_pyproject_toml(content: &str) -> Option<DjangoFmtOptions> {
    let pyproject: PyProject = toml::from_str(content).expect("Failed to parse pyproject.toml");
    let djangofmt = pyproject.tool.and_then(|t| t.djangofmt).unwrap_or_default();
    Some(djangofmt)
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
pub fn load_options<P: AsRef<Path>>(start_path: P) -> Option<DjangoFmtOptions> {
    let pyproject_path =
        find_pyproject_toml(start_path.as_ref()).expect("Failed to find pyproject.toml");
    let content = fs::read_to_string(pyproject_path).ok()?;
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
            Some(DjangoFmtOptions {
                line_length: 200,
                indent_width: 4,
                custom_blocks: vec!["foo".to_string(), "bar".to_string()],
                profile: (Profile::Django)
            })
        );
    }
}

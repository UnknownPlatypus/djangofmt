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

impl TryFrom<&str> for Profile {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "django" => Ok(Self::Django),
            "jinja" => Ok(Self::Jinja),
            _ => Err(format!(
                "Invalid profile: '{s}'. Valid options are 'django' or 'jinja'"
            )),
        }
    }
}

impl FromStr for Profile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
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
fn load_options_from_pyproject_toml(content: &str) -> Result<DjangoFmtOptions, toml::de::Error> {
    let pyproject: PyProject = toml::from_str(content)?;

    // Return the djangofmt config if present, otherwise use defaults
    Ok(pyproject.tool.and_then(|t| t.djangofmt).unwrap_or_default())
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
pub fn load_options<P: AsRef<Path>>(start_path: P) -> Result<DjangoFmtOptions, LoadOptionsError> {
    let pyproject_path =
        find_pyproject_toml(start_path.as_ref()).ok_or(LoadOptionsError::FileNotFound)?;

    let content = fs::read_to_string(&pyproject_path)
        .map_err(|e| LoadOptionsError::IoError(pyproject_path.clone(), e))?;

    load_options_from_pyproject_toml(&content)
        .map_err(|e| LoadOptionsError::ParseError(pyproject_path, e))
}

/// Custom error type for better error handling and reporting
#[derive(Debug)]
pub enum LoadOptionsError {
    /// No pyproject.toml file was found in the directory tree
    FileNotFound,
    /// Failed to read the pyproject.toml file
    IoError(PathBuf, std::io::Error),
    /// Failed to parse the TOML content
    ParseError(PathBuf, toml::de::Error),
}

impl std::fmt::Display for LoadOptionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileNotFound => write!(f, "No pyproject.toml file found"),
            Self::IoError(path, err) => write!(f, "Failed to read {}: {}", path.display(), err),
            Self::ParseError(path, err) => write!(f, "Failed to parse {}: {}", path.display(), err),
        }
    }
}

impl std::error::Error for LoadOptionsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(_, err) => Some(err),
            Self::ParseError(_, err) => Some(err),
            Self::FileNotFound => None,
        }
    }
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
        let result = load_options(&pyproject_path).unwrap();
        assert_eq!(
            result,
            DjangoFmtOptions {
                line_length: 200,
                indent_width: 4,
                custom_blocks: vec!["foo".to_string(), "bar".to_string()],
                profile: Profile::Django
            }
        );
    }

    #[test]
    fn test_load_options_file_not_found() {
        let temp_dir = tempdir().unwrap();
        let result = load_options(temp_dir.path());
        assert!(matches!(result, Err(LoadOptionsError::FileNotFound)));
    }

    #[test]
    fn test_load_options_invalid_toml() {
        let temp_dir = tempdir().unwrap();
        let pyproject_path = temp_dir.path().join("pyproject.toml");
        fs::write(&pyproject_path, "invalid toml content {{{").unwrap();
        let result = load_options(temp_dir.path());
        assert!(matches!(result, Err(LoadOptionsError::ParseError(_, _))));
    }

    #[test]
    fn test_profile_from_str_invalid() {
        let result = Profile::from_str("invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid profile"));
    }

    #[test]
    fn test_profile_from_str_valid() {
        assert_eq!(Profile::from_str("django").unwrap(), Profile::Django);
        assert_eq!(Profile::from_str("jinja").unwrap(), Profile::Jinja);
    }
}

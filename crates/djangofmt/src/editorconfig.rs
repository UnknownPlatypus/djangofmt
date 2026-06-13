use editorconfig_parser::{EditorConfig, EditorConfigProperty, MaxLineLength};
use std::{fs, path::Path};
use tracing::{debug, warn};

use crate::line_width::{IndentWidth, LineLength};

/// Indent/line settings resolved from an `.editorconfig` for a specific file.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct EditorconfigSettings {
    pub line_length: Option<LineLength>,
    pub indent_width: Option<IndentWidth>,
}

/// Parse the nearest `.editorconfig` searching upward from the current working directory.
#[must_use]
pub fn load_editorconfig_from_cwd() -> Option<EditorConfig> {
    load_editorconfig(crate::fs::get_cwd())
}

/// Parse the nearest `.editorconfig` searching upward from `start_path`.
///
/// Section globs are anchored at the file's own directory, so the parsed config
/// is resolved against each formatted file's real path via [`settings_for`].
pub fn load_editorconfig<P: AsRef<Path>>(start_path: P) -> Option<EditorConfig> {
    let Some(path) = crate::fs::find_nearest_ancestor_file(start_path.as_ref(), ".editorconfig")
    else {
        debug!(
            "No .editorconfig found starting search from: {}",
            start_path.as_ref().display()
        );
        return None;
    };
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) => {
            warn!("Failed to read {}: {}", path.display(), err);
            return None;
        }
    };
    debug!("Loading options from .editorconfig at: {}", path.display());
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    Some(EditorConfig::parse(&content).with_cwd(dir))
}

/// Resolve `.editorconfig` indent/line settings for `path`.
#[must_use]
pub fn settings_for(editorconfig: Option<&EditorConfig>, path: &Path) -> EditorconfigSettings {
    let Some(editorconfig) = editorconfig else {
        return EditorconfigSettings::default();
    };
    let properties = editorconfig.resolve(path);

    let mut settings = EditorconfigSettings::default();
    if let EditorConfigProperty::Value(size) = properties.indent_size {
        match IndentWidth::try_from(size) {
            Ok(width) => settings.indent_width = Some(width),
            Err(err) => warn!("Ignoring editorconfig indent_size: {err}"),
        }
    }
    if let EditorConfigProperty::Value(MaxLineLength::Number(length)) = properties.max_line_length {
        match LineLength::try_from(length) {
            Ok(line_length) => settings.line_length = Some(line_length),
            Err(err) => warn!("Ignoring editorconfig max_line_length: {err}"),
        }
    }
    settings
}

/// Whether `.editorconfig` settings can differ between files.
///
/// `false` when there is no config or only a root `[*]` section, letting callers
/// resolve settings once instead of per file.
#[must_use]
pub fn has_per_file_sections(editorconfig: Option<&EditorConfig>) -> bool {
    editorconfig.is_some_and(|editorconfig| {
        let sections = editorconfig.sections();
        !(sections.is_empty() || matches!(sections, [section] if section.name == "*"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Project;
    use rstest::rstest;

    /// Resolve settings for `file` against an `.editorconfig` written at the project root.
    fn settings(editorconfig: &str, file: &str) -> EditorconfigSettings {
        let project = Project::new().file(".editorconfig", editorconfig);
        settings_for(
            load_editorconfig(project.path()).as_ref(),
            &project.join(file),
        )
    }

    #[test]
    fn returns_default_when_no_editorconfig() {
        let project = Project::new();
        assert_eq!(
            settings_for(
                load_editorconfig(project.path()).as_ref(),
                &project.join("x.html")
            ),
            EditorconfigSettings::default()
        );
    }

    #[test]
    fn reads_indent_and_line_length() {
        assert_eq!(
            settings(
                "
root = true

[*]
indent_size = 2
max_line_length = 100
",
                "x.html"
            ),
            EditorconfigSettings {
                line_length: Some(LineLength::try_from(100u16).unwrap()),
                indent_width: Some(IndentWidth::try_from(2u8).unwrap()),
            }
        );
    }

    #[rstest]
    #[case("x.html", 2)]
    #[case("x.jinja", 3)]
    #[case("x.jinja2", 3)]
    #[case("x.j2", 3)]
    fn section_matches_by_file_extension(#[case] file: &str, #[case] expected_indent: u8) {
        let editorconfig = "
[*.html]
indent_size = 2

[*.{jinja,jinja2,j2}]
indent_size = 3
";
        assert_eq!(
            settings(editorconfig, file),
            EditorconfigSettings {
                line_length: None,
                indent_width: Some(IndentWidth::try_from(expected_indent).unwrap()),
            }
        );
    }

    #[test]
    fn detects_whether_sections_vary_per_file() {
        let none = Project::new();
        assert!(!has_per_file_sections(
            load_editorconfig(none.path()).as_ref()
        ));

        let root_only = Project::new().file(".editorconfig", "[*]\nindent_size = 2");
        assert!(!has_per_file_sections(
            load_editorconfig(root_only.path()).as_ref()
        ));

        let per_extension = Project::new().file(".editorconfig", "[*.html]\nindent_size = 2");
        assert!(has_per_file_sections(
            load_editorconfig(per_extension.path()).as_ref()
        ));
    }

    #[test]
    fn matches_jinja_variant_specific_section() {
        // A section written for `.j2` alone is honored for a `.j2` file.
        assert_eq!(
            settings("[*.j2]\nindent_size = 3", "x.j2"),
            EditorconfigSettings {
                line_length: None,
                indent_width: Some(IndentWidth::try_from(3u8).unwrap()),
            }
        );
    }

    #[test]
    fn nearest_editorconfig_wins() {
        let project = Project::new()
            .file(".editorconfig", "[*]\nindent_size = 2")
            .file("child/.editorconfig", "[*]\nindent_size = 8");
        // Only the nearest file is used, the parent one is not merged in.
        let editorconfig = load_editorconfig(project.join("child"));
        assert_eq!(
            settings_for(editorconfig.as_ref(), &project.join("child/x.html")),
            EditorconfigSettings {
                line_length: None,
                indent_width: Some(IndentWidth::try_from(8u8).unwrap()),
            }
        );
    }

    #[rstest]
    #[case("[*]\nindent_size = 0")]
    #[case("[*]\nindent_size = 17")]
    #[case("[*]\nindent_size = 99999")]
    #[case("[*]\nindent_size = tab")]
    #[case("[*]\nindent_size = unset")]
    #[case("[*]\nmax_line_length = 0")]
    #[case("[*]\nmax_line_length = 321")]
    #[case("[*]\nmax_line_length = off")]
    fn ignores_invalid_values(#[case] editorconfig: &str) {
        assert_eq!(
            settings(editorconfig, "x.html"),
            EditorconfigSettings::default()
        );
    }
}

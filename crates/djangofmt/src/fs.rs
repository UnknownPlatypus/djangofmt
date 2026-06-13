use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Return the current working directory, cached after the first call.
///
/// On WASM (where `std::env::current_dir` is unsupported) and on the unlikely
/// event of a read failure, falls back to `.`.
pub fn get_cwd() -> &'static Path {
    static CWD: OnceLock<PathBuf> = OnceLock::new();
    CWD.get_or_init(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .as_path()
}

/// Convert an absolute path to be relative to the current working directory.
pub fn relativize_path<P: AsRef<Path>>(path: P) -> String {
    let path = path.as_ref();
    let cwd = get_cwd();
    if let Ok(stripped) = path.strip_prefix(cwd) {
        return stripped.display().to_string();
    }
    path.display().to_string()
}

/// Finds the nearest `file_name` by traversing directories upward from `start_path`.
pub fn find_nearest_ancestor_file<P: AsRef<Path>>(
    start_path: P,
    file_name: &str,
) -> Option<PathBuf> {
    start_path
        .as_ref()
        .ancestors()
        .map(|directory| directory.join(file_name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Project;

    #[test]
    fn returns_none_when_absent() {
        let project = Project::new();
        assert_eq!(
            find_nearest_ancestor_file(project.path(), "marker.toml"),
            None
        );
    }

    #[test]
    fn finds_file_in_start_dir() {
        let project = Project::new().file("marker.toml", "");
        assert_eq!(
            find_nearest_ancestor_file(project.path(), "marker.toml"),
            Some(project.join("marker.toml"))
        );
    }

    #[test]
    fn finds_file_in_ancestor_dir() {
        let project = Project::new().file("marker.toml", "").dir("child");
        assert_eq!(
            find_nearest_ancestor_file(project.join("child"), "marker.toml"),
            Some(project.join("marker.toml"))
        );
    }
}

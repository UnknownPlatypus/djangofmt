use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use ignore::types::TypesBuilder;
use tracing::{debug, warn};

use crate::args::FileSelectionArgs;
use crate::error::Error;
use crate::pyproject::PyprojectSettings;

/// Default file patterns to include when discovering files.
pub const DEFAULT_INCLUDE: &[&str] = &["*.html", "*.jinja", "*.jinja2", "*.j2"];

/// Default directory/file patterns to exclude when discovering files.
/// Mirrors ruff's defaults.
pub const DEFAULT_EXCLUDE: &[&str] = &[
    ".bzr",
    ".direnv",
    ".eggs",
    ".git",
    ".git-rewrite",
    ".hg",
    ".mypy_cache",
    ".nox",
    ".pants.d",
    ".pytype",
    ".ruff_cache",
    ".svn",
    ".tox",
    ".venv",
    "__pypackages__",
    "_build",
    "buck-out",
    "dist",
    "node_modules",
    "venv",
];

pub(crate) fn resolve_bool_arg(yes: bool, no: bool) -> Option<bool> {
    match (yes, no) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        (false, false) => None,
        (..) => unreachable!("Clap should make this impossible"),
    }
}

/// Resolved File selection configuration after merging CLI, pyproject, and defaults.
#[derive(Debug)]
pub struct ResolvedDiscoveryConfig {
    pub exclude: Vec<String>,
    pub include: Vec<String>,
    pub respect_gitignore: bool,
    pub force_exclude: bool,
}

impl ResolvedDiscoveryConfig {
    /// Build a resolved config by merging CLI args, pyproject settings, and defaults.
    ///
    /// Precedence (highest to lowest): CLI > pyproject > defaults.
    #[must_use]
    pub fn new(cli: &FileSelectionArgs, pyproject: &PyprojectSettings) -> Self {
        let mut exclude = cli
            .exclude
            .clone()
            .or_else(|| pyproject.exclude.clone())
            .unwrap_or_else(|| DEFAULT_EXCLUDE.iter().map(|s| (*s).to_string()).collect());
        exclude.extend(
            pyproject
                .extend_exclude
                .iter()
                .chain(cli.extend_exclude.iter())
                .flatten()
                .cloned(),
        );

        let mut include = pyproject
            .include
            .clone()
            .unwrap_or_else(|| DEFAULT_INCLUDE.iter().map(|s| (*s).to_string()).collect());
        include.extend(pyproject.extend_include.iter().flatten().cloned());

        Self {
            exclude,
            include,
            respect_gitignore: resolve_bool_arg(cli.respect_gitignore, cli.no_respect_gitignore)
                .or(pyproject.respect_gitignore)
                .unwrap_or(true),
            force_exclude: resolve_bool_arg(cli.force_exclude, cli.no_force_exclude)
                .or(pyproject.force_exclude)
                .unwrap_or(false),
        }
    }
}

/// Build include types and exclude overrides from the resolved config.
///
/// Returns `(Types, Overrides)` ready to be plugged into a `WalkBuilder`.
fn build_walk_filters(
    root: &Path,
    config: &ResolvedDiscoveryConfig,
) -> Result<(ignore::types::Types, ignore::overrides::Override), Error> {
    let mut types_builder = TypesBuilder::new();
    for pattern in &config.include {
        types_builder
            .add("djangofmt", pattern)
            .map_err(|e| Error::Resolve(format!("Invalid include pattern '{pattern}': {e}")))?;
    }
    types_builder.select("djangofmt");
    let types = types_builder
        .build()
        .map_err(|e| Error::Resolve(format!("Failed to build file types: {e}")))?;

    let mut override_builder = OverrideBuilder::new(root);
    for pattern in &config.exclude {
        override_builder
            .add(&format!("!{pattern}"))
            .map_err(|e| Error::Resolve(format!("Invalid exclude pattern '{pattern}': {e}")))?;
    }
    let overrides = override_builder
        .build()
        .map_err(|e| Error::Resolve(format!("Failed to build exclude overrides: {e}")))?;

    Ok((types, overrides))
}

/// Resolve a list of CLI paths (files and/or directories) into a flat,
/// deduplicated, sorted list of files to process.
pub fn resolve_files(
    paths: &[PathBuf],
    config: &ResolvedDiscoveryConfig,
) -> Result<Vec<PathBuf>, Error> {
    let mut files: Vec<PathBuf> = Vec::with_capacity(paths.len());
    let mut dirs: Vec<&Path> = vec![];

    // Process the provided paths, collecting directories for later recursive processing.
    // Explicit files paths are canonicalized.
    for path in paths {
        if !path.exists() {
            return Err(Error::Resolve(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }
        if path.is_file() {
            let canonical = std::fs::canonicalize(path).map_err(|e| {
                Error::Resolve(format!("Failed to canonicalize {}: {e}", path.display()))
            })?;
            files.push(canonical);
        } else if path.is_dir() {
            dirs.push(path);
        }
    }

    // When force_exclude is enabled, apply exclude patterns to explicitly-passed files too.
    if config.force_exclude
        && !files.is_empty()
        && let Some(root) = dirs
            .first()
            .copied()
            .or_else(|| files.first().and_then(|f| f.parent()))
    {
        let (_, overrides) = build_walk_filters(root, config)?;
        let len_before = files.len();
        files.retain(|file| {
            let dominated = overrides.matched(file, false);
            if dominated.is_ignore() {
                debug!("Force-excluded: {}", file.display());
                false
            } else {
                true
            }
        });
        if files.len() < len_before {
            debug!(
                "Force-exclude removed {} explicitly-passed files",
                len_before - files.len()
            );
        }
    }

    // Walk all directories with a single parallel WalkBuilder.
    if let Some((first, rest)) = dirs.split_first() {
        let (types, overrides) = build_walk_filters(first, config)?;

        let mut builder = WalkBuilder::new(first);
        for dir in rest {
            builder.add(dir);
        }
        if let Ok(cwd) = std::env::current_dir() {
            builder.current_dir(cwd);
        }
        builder
            .standard_filters(config.respect_gitignore)
            .hidden(false)
            .follow_links(true)
            .types(types)
            .overrides(overrides)
            .threads(
                std::thread::available_parallelism()
                    .map_or(1, std::num::NonZeroUsize::get)
                    .min(12),
            );

        let state = WalkFilesState::new();
        let mut visitor_builder = FileVisitorBuilder::new(&state);
        builder.build_parallel().visit(&mut visitor_builder);
        files.extend(state.finish()?);
    }

    files.sort();
    files.dedup();

    debug!("Resolved {} files to process", files.len());
    Ok(files)
}

/// Shared state across all parallel walk visitors.
struct WalkFilesState {
    files: Mutex<(Vec<PathBuf>, Option<Error>)>,
}

impl WalkFilesState {
    const fn new() -> Self {
        Self {
            files: Mutex::new((vec![], None)),
        }
    }

    fn finish(self) -> Result<Vec<PathBuf>, Error> {
        let (files, error) = self.files.into_inner().expect("walk visitor panicked");
        if let Some(err) = error {
            return Err(err);
        }
        Ok(files)
    }
}

struct FileVisitorBuilder<'s> {
    state: &'s WalkFilesState,
}

impl<'s> FileVisitorBuilder<'s> {
    const fn new(state: &'s WalkFilesState) -> Self {
        Self { state }
    }
}

impl<'s> ignore::ParallelVisitorBuilder<'s> for FileVisitorBuilder<'s> {
    fn build(&mut self) -> Box<dyn ignore::ParallelVisitor + 's> {
        Box::new(FileVisitor {
            local_files: vec![],
            local_error: None,
            global: self.state,
        })
    }
}

struct FileVisitor<'s> {
    local_files: Vec<PathBuf>,
    local_error: Option<Error>,
    global: &'s WalkFilesState,
}

impl ignore::ParallelVisitor for FileVisitor<'_> {
    fn visit(&mut self, result: Result<ignore::DirEntry, ignore::Error>) -> ignore::WalkState {
        match result {
            Ok(entry) if entry.file_type().is_some_and(|ft| ft.is_file()) => {
                match std::fs::canonicalize(entry.path()) {
                    Ok(canonical) => {
                        debug!("Discovered: {}", canonical.display());
                        self.local_files.push(canonical);
                    }
                    Err(e) => {
                        self.local_error = Some(Error::Resolve(format!(
                            "Failed to canonicalize {}: {e}",
                            entry.path().display()
                        )));
                        return ignore::WalkState::Quit;
                    }
                }
            }
            Ok(_) => {}
            Err(err) => {
                warn!("Error walking directory: {err}");
            }
        }
        ignore::WalkState::Continue
    }
}

impl Drop for FileVisitor<'_> {
    fn drop(&mut self) {
        let (files, error) = &mut *self.global.files.lock().expect("walk visitor panicked");

        if files.is_empty() {
            *files = std::mem::take(&mut self.local_files);
        } else {
            files.append(&mut self.local_files);
        }

        if error.is_none() {
            *error = self.local_error.take();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::*;

    fn default_cli() -> FileSelectionArgs {
        FileSelectionArgs::default()
    }

    fn default_pyproject() -> PyprojectSettings {
        PyprojectSettings::default()
    }

    /// Helper to create a file in a temp dir
    fn create_file(base: &Path, relative: &str) {
        let path = base.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, "<div>test</div>").unwrap();
    }

    #[test]
    fn test_defaults() {
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject());
        assert_eq!(
            config.include,
            vec!["*.html", "*.jinja", "*.jinja2", "*.j2"]
        );
        assert!(config.exclude.contains(&".git".to_string()));
        assert!(config.exclude.contains(&".venv".to_string()));
        assert!(config.exclude.contains(&"node_modules".to_string()));
        assert!(config.respect_gitignore);
    }

    #[test]
    fn test_pyproject_exclude_replaces_defaults() {
        let pyproject = PyprojectSettings {
            exclude: Some(vec!["custom_dir".to_string()]),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &pyproject);
        assert_eq!(config.exclude, vec!["custom_dir"]);
    }

    #[test]
    fn test_pyproject_extend_exclude_adds_to_defaults() {
        let pyproject = PyprojectSettings {
            extend_exclude: Some(vec!["vendor".to_string()]),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &pyproject);
        assert!(config.exclude.contains(&".git".to_string()));
        assert!(config.exclude.contains(&"vendor".to_string()));
    }

    #[test]
    fn test_cli_exclude_replaces_pyproject_and_defaults() {
        let cli = FileSelectionArgs {
            exclude: Some(vec!["migrations".to_string()]),
            ..Default::default()
        };
        let pyproject = PyprojectSettings {
            exclude: Some(vec!["should_be_replaced".to_string()]),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&cli, &pyproject);
        assert_eq!(config.exclude, vec!["migrations"]);
    }

    #[test]
    fn test_extend_exclude_accumulates_from_both() {
        let cli = FileSelectionArgs {
            extend_exclude: Some(vec!["cli_extra".to_string()]),
            ..Default::default()
        };
        let pyproject = PyprojectSettings {
            extend_exclude: Some(vec!["pyproject_extra".to_string()]),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&cli, &pyproject);
        assert!(config.exclude.contains(&"pyproject_extra".to_string()));
        assert!(config.exclude.contains(&"cli_extra".to_string()));
    }

    #[test]
    fn test_cli_exclude_with_extend_exclude() {
        let cli = FileSelectionArgs {
            exclude: Some(vec!["migrations".to_string()]),
            extend_exclude: Some(vec!["vendor".to_string()]),
            ..Default::default()
        };
        let pyproject = PyprojectSettings {
            extend_exclude: Some(vec!["build".to_string()]),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&cli, &pyproject);
        assert!(config.exclude.contains(&"migrations".to_string()));
        assert!(config.exclude.contains(&"build".to_string()));
        assert!(config.exclude.contains(&"vendor".to_string()));
        assert!(!config.exclude.contains(&".git".to_string()));
    }

    #[test]
    fn test_pyproject_include_replaces_defaults() {
        let pyproject = PyprojectSettings {
            include: Some(vec!["*.txt".to_string()]),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &pyproject);
        assert_eq!(config.include, vec!["*.txt"]);
    }

    #[test]
    fn test_pyproject_extend_include_adds_to_defaults() {
        let pyproject = PyprojectSettings {
            extend_include: Some(vec!["*.djhtml".to_string()]),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &pyproject);
        assert_eq!(
            config.include,
            vec!["*.html", "*.jinja", "*.jinja2", "*.j2", "*.djhtml"]
        );
    }

    #[test]
    fn test_respect_gitignore_default_true() {
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject());
        assert!(config.respect_gitignore);
    }

    #[test]
    fn test_respect_gitignore_pyproject_false() {
        let pyproject = PyprojectSettings {
            respect_gitignore: Some(false),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &pyproject);
        assert!(!config.respect_gitignore);
    }

    #[test]
    fn test_respect_gitignore_cli_overrides_pyproject() {
        let cli = FileSelectionArgs {
            no_respect_gitignore: true,
            ..Default::default()
        };
        let pyproject = PyprojectSettings {
            respect_gitignore: Some(true),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&cli, &pyproject);
        assert!(!config.respect_gitignore);
    }

    #[test]
    fn test_resolve_files_discovers_html_in_directory() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), "a.html");
        create_file(dir.path(), "b.jinja");
        create_file(dir.path(), "c.jinja2");
        create_file(dir.path(), "d.py");
        create_file(dir.path(), "e.css");

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"a.html".to_string()));
        assert!(names.contains(&"b.jinja".to_string()));
        assert!(names.contains(&"c.jinja2".to_string()));
        assert!(!names.contains(&"d.py".to_string()));
        assert!(!names.contains(&"e.css".to_string()));
    }

    #[test]
    fn test_resolve_files_explicit_file_bypasses_excludes() {
        let dir = tempdir().unwrap();
        let excluded_dir = dir.path().join(".venv");
        fs::create_dir_all(&excluded_dir).unwrap();
        create_file(dir.path(), ".venv/template.html");

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject());
        let explicit_file = excluded_dir.join("template.html");
        let files = resolve_files(&[explicit_file], &config).unwrap();

        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_resolve_files_directory_respects_excludes() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), "good.html");
        create_file(dir.path(), ".venv/bad.html");

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"good.html".to_string()));
        assert!(!names.contains(&"bad.html".to_string()));
    }

    #[test]
    fn test_resolve_files_respects_gitignore() {
        let dir = tempdir().unwrap();
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        fs::write(dir.path().join(".gitignore"), "ignored/\n").unwrap();
        create_file(dir.path(), "included.html");
        create_file(dir.path(), "ignored/excluded.html");

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"included.html".to_string()));
        assert!(!names.contains(&"excluded.html".to_string()));
    }

    #[test]
    fn test_resolve_files_no_respect_gitignore() {
        let dir = tempdir().unwrap();
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        fs::write(dir.path().join(".gitignore"), "ignored/\n").unwrap();
        create_file(dir.path(), "included.html");
        create_file(dir.path(), "ignored/also_included.html");

        let pyproject = PyprojectSettings {
            respect_gitignore: Some(false),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &pyproject);
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"included.html".to_string()));
        assert!(names.contains(&"also_included.html".to_string()));
    }

    #[test]
    fn test_resolve_files_empty_directory() {
        let dir = tempdir().unwrap();
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_resolve_files_nonexistent_path_errors() {
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject());
        let result = resolve_files(&[PathBuf::from("/nonexistent/path")], &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_files_nested_directories() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), "top.html");
        create_file(dir.path(), "sub/nested.html");
        create_file(dir.path(), "sub/deep/deeper.jinja2");

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_resolve_files_custom_include() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), "a.html");
        create_file(dir.path(), "b.txt");

        let pyproject = PyprojectSettings {
            include: Some(vec!["*.txt".to_string()]),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &pyproject);
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(!names.contains(&"a.html".to_string()));
        assert!(names.contains(&"b.txt".to_string()));
    }

    #[test]
    fn test_resolve_files_deduplicates() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), "a.html");

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject());
        let explicit = dir.path().join("a.html");
        let files = resolve_files(&[dir.path().to_path_buf(), explicit], &config).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_resolve_files_sorted_deterministically() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), "c.html");
        create_file(dir.path(), "a.html");
        create_file(dir.path(), "b.html");

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["a.html", "b.html", "c.html"]);
    }

    #[test]
    fn test_resolve_files_invalid_glob_pattern_errors() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), "a.html");

        let pyproject = PyprojectSettings {
            include: Some(vec!["[invalid".to_string()]),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &pyproject);
        let result = resolve_files(&[dir.path().to_path_buf()], &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_files_nested_gitignore() {
        let dir = tempdir().unwrap();
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        fs::write(dir.path().join(".gitignore"), "").unwrap();
        fs::create_dir_all(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/.gitignore"), "ignored_nested/\n").unwrap();
        create_file(dir.path(), "sub/included.html");
        create_file(dir.path(), "sub/ignored_nested/excluded.html");

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"included.html".to_string()));
        assert!(!names.contains(&"excluded.html".to_string()));
    }

    #[test]
    fn test_resolve_files_force_exclude_filters_explicit_files() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), "a.html");
        create_file(dir.path(), "b.html");

        let cli = FileSelectionArgs {
            force_exclude: true,
            exclude: Some(vec!["b.html".to_string()]),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&cli, &default_pyproject());
        let files = resolve_files(
            &[dir.path().join("a.html"), dir.path().join("b.html")],
            &config,
        )
        .unwrap();

        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"a.html".to_string()));
        assert!(!names.contains(&"b.html".to_string()));
    }

    #[test]
    fn test_resolve_files_no_force_exclude_keeps_explicit_files() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), "a.html");
        create_file(dir.path(), "b.html");

        let cli = FileSelectionArgs {
            exclude: Some(vec!["b.html".to_string()]),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&cli, &default_pyproject());
        let files = resolve_files(
            &[dir.path().join("a.html"), dir.path().join("b.html")],
            &config,
        )
        .unwrap();

        assert_eq!(files.len(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn test_resolve_files_follows_symlinks() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), "real_dir/template.html");
        std::os::unix::fs::symlink(dir.path().join("real_dir"), dir.path().join("link_dir"))
            .unwrap();

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        assert!(
            files
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                .any(|x| x == "template.html")
        );
    }
}

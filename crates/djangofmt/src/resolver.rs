use std::path::{Path, PathBuf};
use std::sync::Mutex;

use globset::{Glob, GlobSet, GlobSetBuilder, escape};
use ignore::WalkBuilder;
use tracing::{debug, warn};

use crate::args::FileSelectionArgs;
use crate::config::resolve_bool_arg;
use crate::error::Error;
use crate::pyproject::PyprojectSettings;

/// Default file patterns to include when discovering files.
pub const DEFAULT_INCLUDE: &[&str] = &["*.html", "*.jinja", "*.jinja2", "*.j2"];

/// Default directory/file patterns to exclude when discovering files.
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

/// Resolved File selection configuration after merging CLI, pyproject, and defaults.
#[derive(Debug)]
pub struct ResolvedDiscoveryConfig {
    /// List of file path patterns to exclude.
    pub exclude: Vec<String>,
    /// List of file path patterns to include.
    pub include: Vec<String>,
    /// Respect `.gitignore` files when discovering files
    pub respect_gitignore: bool,
    /// Enforce exclusions, even for paths passed to djangofmt directly on the command-line.
    pub force_exclude: bool,
    /// Anchor for path-relative `include`/`exclude` patterns (the `pyproject.toml` directory).
    pub project_root: PathBuf,
}

impl ResolvedDiscoveryConfig {
    /// Build a resolved config by merging CLI args, pyproject settings, and defaults.
    ///
    /// Precedence (highest to lowest): CLI > pyproject > defaults.
    #[must_use]
    pub fn new(
        cli: &FileSelectionArgs,
        pyproject: &PyprojectSettings,
        project_root: &Path,
    ) -> Self {
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
            project_root: project_root
                .canonicalize()
                .unwrap_or_else(|_| project_root.to_path_buf()),
        }
    }
}

/// Include or exclude patterns compiled into the two matchers of
/// [`crate::per_file_ignores`]: a bare pattern matches a path's name at any depth, a path
/// pattern is anchored at the project root.
struct PathMatcher {
    basenames: GlobSet,
    paths: GlobSet,
}

impl PathMatcher {
    fn new(patterns: &[String], project_root: &Path, kind: &str) -> Result<Self, Error> {
        let escaped_root = escape(&project_root.to_string_lossy());
        let compile = |glob: &str, pattern: &str| {
            Glob::new(glob)
                .map_err(|e| Error::Resolve(format!("Invalid {kind} pattern '{pattern}': {e}")))
        };
        let mut basenames = GlobSetBuilder::new();
        let mut paths = GlobSetBuilder::new();
        for pattern in patterns {
            if pattern.starts_with('!') {
                return Err(Error::Resolve(format!(
                    "Negated {kind} pattern '{pattern}' is not supported"
                )));
            }
            // A trailing slash means "directory only" in gitignore; directories are matched
            // either way here, so drop it rather than let the pattern match nothing.
            let glob = pattern.trim_end_matches('/');
            if glob.contains('/') {
                paths.add(compile(
                    &crate::fs::anchor_glob(&escaped_root, glob),
                    pattern,
                )?);
            } else {
                basenames.add(compile(glob, pattern)?);
            }
        }
        let build = |builder: GlobSetBuilder| {
            builder
                .build()
                .map_err(|e| Error::Resolve(format!("Failed to build {kind} patterns: {e}")))
        };
        Ok(Self {
            basenames: build(basenames)?,
            paths: build(paths)?,
        })
    }

    fn is_match(&self, path: &Path) -> bool {
        path.file_name()
            .is_some_and(|name| self.basenames.is_match(name))
            || self.paths.is_match(path)
    }

    /// Whether `path` or any ancestor up to `project_root` matches. The walk prunes excluded
    /// directories, but explicitly-passed paths never reach it, so they check parents here.
    /// Mirrors ruff's `is_file_excluded`.
    fn matches_ancestor(&self, path: &Path, project_root: &Path) -> bool {
        for ancestor in path.ancestors() {
            if self.is_match(ancestor) {
                return true;
            }
            if ancestor == project_root {
                break;
            }
        }
        false
    }
}

/// Return `true` if the given filename should be force-excluded based on the resolved configuration.
/// Returns `false` if `force_exclude` is disabled.
pub fn is_force_excluded(filename: &Path, config: &ResolvedDiscoveryConfig) -> Result<bool, Error> {
    if !config.force_exclude {
        return Ok(false);
    }
    let exclude = PathMatcher::new(&config.exclude, &config.project_root, "exclude")?;
    // Stdin filenames may be relative to the cwd and may not exist on disk.
    let path = crate::fs::get_cwd().join(filename);
    Ok(exclude.matches_ancestor(&path.canonicalize().unwrap_or(path), &config.project_root))
}

/// Resolve a list of CLI paths (files and/or directories) into a flat,
/// deduplicated, sorted list of files to process.
pub fn resolve_files(
    paths: &[PathBuf],
    config: &ResolvedDiscoveryConfig,
) -> Result<Vec<PathBuf>, Error> {
    let mut files: Vec<PathBuf> = Vec::with_capacity(paths.len());
    let mut dirs: Vec<PathBuf> = vec![];

    // Process the provided paths, collecting directories for later recursive processing.
    // Paths are canonicalized so they match patterns anchored at the canonical project root.
    for path in paths {
        if !path.exists() {
            return Err(Error::Resolve(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }
        let canonical = std::fs::canonicalize(path).map_err(|e| {
            Error::Resolve(format!("Failed to canonicalize {}: {e}", path.display()))
        })?;
        if canonical.is_file() {
            files.push(canonical);
        } else if canonical.is_dir() {
            dirs.push(canonical);
        }
    }

    let exclude = PathMatcher::new(&config.exclude, &config.project_root, "exclude")?;

    // When force_exclude is enabled, apply exclude patterns to explicitly-passed paths too.
    if config.force_exclude {
        let retain = |paths: &mut Vec<PathBuf>| {
            paths.retain(|path| {
                let excluded = exclude.matches_ancestor(path, &config.project_root);
                if excluded {
                    debug!("Force-excluded: {}", path.display());
                }
                !excluded
            });
        };
        retain(&mut files);
        retain(&mut dirs);
    }

    // Walk all directories with a single parallel WalkBuilder.
    if let Some((first, rest)) = dirs.split_first() {
        let include = PathMatcher::new(&config.include, &config.project_root, "include")?;

        let mut builder = WalkBuilder::new(first);
        for dir in rest {
            builder.add(dir);
        }
        builder.current_dir(crate::fs::get_cwd());
        builder
            .standard_filters(config.respect_gitignore)
            .hidden(false)
            .follow_links(true)
            .filter_entry(move |entry| {
                // Roots come from the command line, so only the walk's own entries are excluded.
                if entry.depth() > 0 && exclude.is_match(entry.path()) {
                    debug!("Excluded: {}", entry.path().display());
                    return false;
                }
                // Prune non-template files here so they never reach a visitor.
                !entry.file_type().is_some_and(|ft| ft.is_file()) || include.is_match(entry.path())
            })
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
            canonical_parent: None,
        })
    }
}

struct FileVisitor<'s> {
    local_files: Vec<PathBuf>,
    local_error: Option<Error>,
    global: &'s WalkFilesState,
    /// Last `(directory, canonical directory)` pair, reused across the entries of a
    /// directory since the walker hands them to a single visitor.
    canonical_parent: Option<(PathBuf, PathBuf)>,
}

impl FileVisitor<'_> {
    /// Canonicalize a walked file path, resolving its parent directory at most once per
    /// directory. Walk roots are already canonical, so only symlinks need resolving.
    fn canonicalize(&mut self, entry: &ignore::DirEntry) -> std::io::Result<PathBuf> {
        let path = entry.path();
        let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
            return std::fs::canonicalize(path);
        };
        if entry.path_is_symlink() {
            return std::fs::canonicalize(path);
        }
        if self
            .canonical_parent
            .as_ref()
            .is_none_or(|(dir, _)| dir != parent)
        {
            self.canonical_parent = Some((parent.to_path_buf(), std::fs::canonicalize(parent)?));
        }
        let (_, canonical_parent) = self.canonical_parent.as_ref().expect("just inserted");
        Ok(canonical_parent.join(name))
    }
}

impl ignore::ParallelVisitor for FileVisitor<'_> {
    fn visit(&mut self, result: Result<ignore::DirEntry, ignore::Error>) -> ignore::WalkState {
        match result {
            Ok(entry) if entry.file_type().is_some_and(|ft| ft.is_file()) => {
                match self.canonicalize(&entry) {
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

    /// Config for the merge-precedence tests, which never discover files so we pass a dummy anchor.
    fn resolved(cli: &FileSelectionArgs, pyproject: &PyprojectSettings) -> ResolvedDiscoveryConfig {
        ResolvedDiscoveryConfig::new(cli, pyproject, Path::new("."))
    }

    /// Helper to create a file in a temp dir
    fn create_file(base: &Path, relative: &str) {
        let path = base.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, "<div>test</div>").unwrap();
    }

    fn file_names(files: &[PathBuf]) -> Vec<String> {
        files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect()
    }

    #[test]
    fn test_defaults() {
        let config = resolved(&default_cli(), &default_pyproject());
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
        let config = resolved(&default_cli(), &pyproject);
        assert_eq!(config.exclude, vec!["custom_dir"]);
    }

    #[test]
    fn test_pyproject_extend_exclude_adds_to_defaults() {
        let pyproject = PyprojectSettings {
            extend_exclude: Some(vec!["vendor".to_string()]),
            ..Default::default()
        };
        let config = resolved(&default_cli(), &pyproject);
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
        let config = resolved(&cli, &pyproject);
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
        let config = resolved(&cli, &pyproject);
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
        let config = resolved(&cli, &pyproject);
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
        let config = resolved(&default_cli(), &pyproject);
        assert_eq!(config.include, vec!["*.txt"]);
    }

    #[test]
    fn test_pyproject_extend_include_adds_to_defaults() {
        let pyproject = PyprojectSettings {
            extend_include: Some(vec!["*.djhtml".to_string()]),
            ..Default::default()
        };
        let config = resolved(&default_cli(), &pyproject);
        assert_eq!(
            config.include,
            vec!["*.html", "*.jinja", "*.jinja2", "*.j2", "*.djhtml"]
        );
    }

    #[test]
    fn test_respect_gitignore_pyproject_false() {
        let pyproject = PyprojectSettings {
            respect_gitignore: Some(false),
            ..Default::default()
        };
        let config = resolved(&default_cli(), &pyproject);
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
        let config = resolved(&cli, &pyproject);
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

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject(), dir.path());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        let names = file_names(&files);
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

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject(), dir.path());
        let explicit_file = excluded_dir.join("template.html");
        let files = resolve_files(&[explicit_file], &config).unwrap();

        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_resolve_files_directory_respects_excludes() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), "good.html");
        create_file(dir.path(), ".venv/bad.html");

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject(), dir.path());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        let names = file_names(&files);
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

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject(), dir.path());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        let names = file_names(&files);
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
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &pyproject, dir.path());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        let names = file_names(&files);
        assert!(names.contains(&"included.html".to_string()));
        assert!(names.contains(&"also_included.html".to_string()));
    }

    #[test]
    fn test_resolve_files_empty_directory() {
        let dir = tempdir().unwrap();
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject(), dir.path());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_resolve_files_nonexistent_path_errors() {
        let config = resolved(&default_cli(), &default_pyproject());
        let result = resolve_files(&[PathBuf::from("/nonexistent/path")], &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_files_nested_directories() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), "top.html");
        create_file(dir.path(), "sub/nested.html");
        create_file(dir.path(), "sub/deep/deeper.jinja2");

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject(), dir.path());
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
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &pyproject, dir.path());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        let names = file_names(&files);
        assert!(!names.contains(&"a.html".to_string()));
        assert!(names.contains(&"b.txt".to_string()));
    }

    /// Worked example of ruff's `include` glob semantics: path patterns anchor at the
    /// project root and `*` crosses `/`, while a bare pattern matches at any depth.
    #[test]
    fn test_resolve_files_include_glob_semantics() {
        let dir = tempdir().unwrap();
        for path in [
            "root.html",
            "templates/page.html",
            "templates/deep/page.html",
            "templates/rnested.html",
            "other/page.html",
        ] {
            create_file(dir.path(), path);
        }
        let root = dir.path().canonicalize().unwrap();

        let matched = |pattern: &str| {
            let pyproject = PyprojectSettings {
                include: Some(vec![pattern.to_string()]),
                ..Default::default()
            };
            let config = ResolvedDiscoveryConfig::new(&default_cli(), &pyproject, dir.path());
            let mut found: Vec<String> = resolve_files(&[dir.path().to_path_buf()], &config)
                .unwrap()
                .iter()
                .map(|p| p.strip_prefix(&root).unwrap().display().to_string())
                .collect();
            found.sort();
            found
        };

        // Bare pattern: matches the file name at any depth.
        assert_eq!(
            matched("*.html"),
            [
                "other/page.html",
                "root.html",
                "templates/deep/page.html",
                "templates/page.html",
                "templates/rnested.html"
            ]
        );
        assert_eq!(matched("r*.html"), ["root.html", "templates/rnested.html"]);
        // Path pattern: anchored at the root, and `*` crosses `/` so nested files match too.
        assert_eq!(
            matched("templates/*.html"),
            [
                "templates/deep/page.html",
                "templates/page.html",
                "templates/rnested.html"
            ]
        );
        // `**/` still spans directories.
        assert_eq!(matched("**/deep/*.html"), ["templates/deep/page.html"]);
    }

    #[test]
    fn test_resolve_files_exclude_glob_semantics() {
        let dir = tempdir().unwrap();
        for path in [
            "root.html",
            "sub/page.html",
            "sub/deep/page.html",
            "other/sub/page.html",
            "other/page.html",
        ] {
            create_file(dir.path(), path);
        }
        let root = dir.path().canonicalize().unwrap();

        let kept = |pattern: &str| {
            let pyproject = PyprojectSettings {
                exclude: Some(vec![pattern.to_string()]),
                ..Default::default()
            };
            let config = ResolvedDiscoveryConfig::new(&default_cli(), &pyproject, dir.path());
            let mut found: Vec<String> = resolve_files(&[dir.path().to_path_buf()], &config)
                .unwrap()
                .iter()
                .map(|p| p.strip_prefix(&root).unwrap().display().to_string())
                .collect();
            found.sort();
            found
        };

        // Bare pattern: matches the name at any depth, directories included.
        assert_eq!(kept("sub"), ["other/page.html", "root.html"]);
        // A trailing slash keeps that meaning instead of matching nothing.
        assert_eq!(kept("sub/"), ["other/page.html", "root.html"]);
        // Path pattern: anchored at the root, and `*` crosses `/` so nested files match too.
        assert_eq!(
            kept("sub/*.html"),
            ["other/page.html", "other/sub/page.html", "root.html"]
        );
        // Absolute patterns are matched as-is.
        assert_eq!(
            kept(&root.join("sub").display().to_string()),
            ["other/page.html", "other/sub/page.html", "root.html"]
        );
    }

    /// The walk and explicitly-passed paths share one matcher, so a pattern must not
    /// select differently depending on how the file was reached.
    #[test]
    fn test_resolve_files_exclude_agrees_across_walk_and_explicit_paths() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), "sub/deep/page.html");

        let cli = FileSelectionArgs {
            force_exclude: true,
            exclude: Some(vec!["sub/*.html".to_string()]),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&cli, &default_pyproject(), dir.path());

        assert!(
            resolve_files(&[dir.path().to_path_buf()], &config)
                .unwrap()
                .is_empty()
        );
        assert!(
            resolve_files(&[dir.path().join("sub/deep/page.html")], &config)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_resolve_files_negated_exclude_pattern_errors() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), "keep.html");

        let pyproject = PyprojectSettings {
            exclude: Some(vec!["*.html".to_string(), "!keep.html".to_string()]),
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &pyproject, dir.path());
        let err = resolve_files(&[dir.path().to_path_buf()], &config).unwrap_err();

        assert!(
            err.to_string()
                .contains("Negated exclude pattern '!keep.html' is not supported"),
            "{err}"
        );
    }

    #[test]
    fn test_resolve_files_deduplicates() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), "a.html");

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject(), dir.path());
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

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject(), dir.path());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        let names = file_names(&files);
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
        let config = ResolvedDiscoveryConfig::new(&default_cli(), &pyproject, dir.path());
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

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject(), dir.path());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        let names = file_names(&files);
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
        let config = ResolvedDiscoveryConfig::new(&cli, &default_pyproject(), dir.path());
        let files = resolve_files(
            &[dir.path().join("a.html"), dir.path().join("b.html")],
            &config,
        )
        .unwrap();

        let names = file_names(&files);
        assert!(names.contains(&"a.html".to_string()));
        assert!(!names.contains(&"b.html".to_string()));
    }

    #[test]
    fn test_resolve_files_force_exclude_outside_project_root() {
        // Outside the root, root-anchored patterns can't apply but name ones still do.
        let root = tempdir().unwrap();
        let outside = tempdir().unwrap();
        create_file(outside.path(), ".venv/a.html");
        create_file(outside.path(), "b.html");

        let cli = FileSelectionArgs {
            force_exclude: true,
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&cli, &default_pyproject(), root.path());
        let files = resolve_files(
            &[
                outside.path().join(".venv/a.html"),
                outside.path().join("b.html"),
            ],
            &config,
        )
        .unwrap();

        assert_eq!(file_names(&files), vec!["b.html"]);
    }

    #[test]
    fn test_resolve_files_force_exclude_filters_explicit_directories() {
        let dir = tempdir().unwrap();
        create_file(dir.path(), ".venv/lib/a.html");

        let cli = FileSelectionArgs {
            force_exclude: true,
            ..Default::default()
        };
        let config = ResolvedDiscoveryConfig::new(&cli, &default_pyproject(), dir.path());
        let files = resolve_files(&[dir.path().join(".venv/lib")], &config).unwrap();

        assert!(files.is_empty());
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
        let config = ResolvedDiscoveryConfig::new(&cli, &default_pyproject(), dir.path());
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
        let target = dir.path().join("real_dir/template.html");
        std::os::unix::fs::symlink(dir.path().join("real_dir"), dir.path().join("link_dir"))
            .unwrap();
        std::os::unix::fs::symlink(&target, dir.path().join("link_template.html")).unwrap();

        let config = ResolvedDiscoveryConfig::new(&default_cli(), &default_pyproject(), dir.path());
        let files = resolve_files(&[dir.path().to_path_buf()], &config).unwrap();

        // Both links resolve to the single real file.
        assert_eq!(files, vec![target.canonicalize().unwrap()]);
    }
}

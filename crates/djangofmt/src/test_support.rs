//! Shared filesystem helpers for tests.
#![allow(dead_code)] // Each test binary uses only a subset of these helpers.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// A throwaway directory of files for a test; cleaned up on drop.
///
/// ```ignore
/// let project = Project::new().file("pyproject.toml", "[tool.djangofmt]\n");
/// load_options(&project.join("pyproject.toml"));
/// ```
pub struct Project(TempDir);

impl Project {
    pub fn new() -> Self {
        Self(TempDir::new().expect("create temp dir"))
    }

    /// Write `content` to `name` (creating parent dirs), returning `self` to chain.
    pub fn file(self, name: &str, content: &str) -> Self {
        let path = self.0.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        fs::write(path, content).expect("write project file");
        self
    }

    /// Create an empty subdirectory `name`, returning `self` to chain.
    pub fn dir(self, name: &str) -> Self {
        fs::create_dir_all(self.0.path().join(name)).expect("create dir");
        self
    }

    pub fn path(&self) -> &Path {
        self.0.path()
    }

    pub fn join(&self, name: &str) -> PathBuf {
        self.0.path().join(name)
    }

    pub fn read(&self, name: &str) -> String {
        fs::read_to_string(self.join(name)).expect("read project file")
    }
}

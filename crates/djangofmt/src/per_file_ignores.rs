//! Per-file rule ignores: drop selected lint rules for files matching a glob.
//!
//! Ruff equivalent: `ruff_linter`'s `per-file-ignores` / `CompiledPerFileIgnoreList`.
//!
//! Configured under `[tool.djangofmt.lint.per-file-ignores]` as a map from a glob
//! to the rule selectors to ignore for matching files. A file's effective rule set
//! is the global selection minus the union of every matching glob's ignored rules.

use std::path::{Path, PathBuf};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use rustc_hash::FxHashMap;

use djangofmt_lint::{RuleSelector, RuleSet};

use crate::error::{Error, Result};

/// Per-file-ignore globs compiled into a single matcher, paired index-for-index
/// with the rule set each glob removes.
#[derive(Debug)]
pub struct PerFileIgnores {
    set: GlobSet,
    /// Rules to ignore, parallel to the globs registered in `set`.
    ignored: Vec<RuleSet>,
    /// Project root the globs are anchored at, canonical to match discovered files.
    root: PathBuf,
}

impl PerFileIgnores {
    /// Compile `patterns` (glob -> selectors to ignore) anchored at `root`.
    ///
    /// A bare pattern (no `/`) matches the file's basename at any depth; a pattern
    /// with a separator is anchored at the project root. Mirrors ruff's behavior.
    pub fn new(patterns: &FxHashMap<String, Vec<RuleSelector>>, root: &Path) -> Result<Self> {
        let mut builder = GlobSetBuilder::new();
        let mut ignored = Vec::with_capacity(patterns.len());
        for (pattern, selectors) in patterns {
            let anchored = if pattern.contains('/') {
                pattern.clone()
            } else {
                format!("**/{pattern}")
            };
            let glob = GlobBuilder::new(&anchored)
                .literal_separator(true)
                .build()
                .map_err(|e| {
                    Error::Resolve(format!("Invalid per-file-ignores pattern '{pattern}': {e}"))
                })?;
            builder.add(glob);
            ignored.push(selectors.iter().flat_map(|s| s.all_rules()).collect());
        }
        let set = builder
            .build()
            .map_err(|e| Error::Resolve(format!("Failed to build per-file-ignores: {e}")))?;
        Ok(Self {
            set,
            ignored,
            // Files are discovered as canonical paths; canonicalize the root so the
            // glob candidate (root-relative path) strips cleanly. Fall back to the
            // raw root if it can't be canonicalized.
            root: root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
        })
    }

    /// Effective rule set for `path`: `base` minus every matching glob's ignored rules.
    #[must_use]
    pub fn rules_for(&self, path: &Path, base: &RuleSet) -> RuleSet {
        let candidate = path.strip_prefix(&self.root).unwrap_or(path);
        let mut rules = *base;
        for idx in self.set.matches(candidate) {
            for rule in &self.ignored[idx] {
                rules.remove(rule);
            }
        }
        rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use djangofmt_lint::Rule;

    fn all_rules() -> RuleSet {
        RuleSelector::All.all_rules().collect()
    }

    /// Worked example: a bare glob ignores by basename at any depth, a path glob is
    /// anchored at the root, ignores union across matches, and a rule listed for one
    /// glob is untouched on files that don't match it.
    #[test]
    fn per_file_ignores_worked_example() {
        let root = Path::new("/proj");
        let mut patterns = FxHashMap::default();
        patterns.insert(
            "*.jinja".to_string(),
            vec![RuleSelector::Rule(Rule::UseHttps)],
        );
        patterns.insert(
            "templates/admin/**".to_string(),
            vec![RuleSelector::Rule(Rule::InvalidAttrValue)],
        );
        let pfi = PerFileIgnores::new(&patterns, root).unwrap();
        let base = all_rules();

        // Bare glob: a `.jinja` nested anywhere drops `use-https` only.
        let nested = pfi.rules_for(Path::new("/proj/app/templates/x.jinja"), &base);
        assert!(!nested.contains(Rule::UseHttps));
        assert!(nested.contains(Rule::InvalidAttrValue));

        // A root-level admin `.jinja` matches both globs: the ignore sets union.
        let admin = pfi.rules_for(Path::new("/proj/templates/admin/page.jinja"), &base);
        assert!(!admin.contains(Rule::UseHttps)); // from `*.jinja`
        assert!(!admin.contains(Rule::InvalidAttrValue)); // from `templates/admin/**`

        // `templates/admin/**` is anchored: the same dir nested under `app/` doesn't match.
        let not_admin = pfi.rules_for(Path::new("/proj/app/templates/admin/page.html"), &base);
        assert!(not_admin.contains(Rule::InvalidAttrValue));

        // A file matching nothing keeps the full set.
        assert_eq!(pfi.rules_for(Path::new("/proj/index.html"), &base), base);
    }

    #[test]
    fn invalid_pattern_errors() {
        let mut patterns = FxHashMap::default();
        patterns.insert("[unclosed".to_string(), vec![]);
        assert!(PerFileIgnores::new(&patterns, Path::new("/proj")).is_err());
    }
}

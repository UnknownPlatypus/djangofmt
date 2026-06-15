//! Per-file rule ignores: drop selected lint rules for files matching a glob.
//!
//! Ruff equivalent: `ruff_linter`'s `per-file-ignores` / `CompiledPerFileIgnoreList`.
//!
//! Configured under `[tool.djangofmt.lint.per-file-ignores]` as a map from a glob
//! to the rule selectors to ignore for matching files. A file's effective rule set
//! is the global selection minus the union of every matching glob's ignored rules.
//!
//! Glob semantics follow ruff exactly, so config ports over unchanged: every pattern
//! is matched both against the file's basename and against the pattern normalized to
//! the project root, and `*` crosses `/` (`templates/*.html` also covers nested files).
//! Ruff tests its patterns one matcher at a time; we compile each of the two families
//! into a `GlobSet`, so match cost does not grow with the number of patterns.

use std::path::Path;

use globset::{Candidate, Glob, GlobSet, GlobSetBuilder, escape};
use std::collections::BTreeMap;

use djangofmt_lint::{RuleSelector, RuleSet};

use crate::error::{Error, Result};

/// Per-file-ignore globs compiled into two matchers, each paired index-for-index
/// with the rule set its glob removes.
#[derive(Debug)]
pub struct PerFileIgnores {
    /// Patterns matched against a file's basename.
    basenames: GlobSet,
    /// The same patterns anchored at the project root, matched against the full path.
    absolutes: GlobSet,
    /// Rules to ignore, parallel to the globs registered in both sets.
    ignored: Vec<RuleSet>,
}

impl PerFileIgnores {
    /// Compile `patterns` (glob -> selectors to ignore) anchored at `root`.
    pub fn new(patterns: &BTreeMap<String, Vec<RuleSelector>>, root: &Path) -> Result<Self> {
        // Files are discovered as canonical paths, so anchor the globs at the canonical
        // root. Fall back to the raw root if it can't be canonicalized.
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let escaped_root = escape(&root.to_string_lossy());

        let mut basenames = GlobSetBuilder::new();
        let mut absolutes = GlobSetBuilder::new();
        let mut ignored = Vec::with_capacity(patterns.len());
        for (pattern, selectors) in patterns {
            let anchored = if Path::new(pattern).is_absolute() {
                pattern.clone()
            } else {
                format!("{escaped_root}/{pattern}")
            };
            basenames.add(compile(pattern, pattern)?);
            absolutes.add(compile(&anchored, pattern)?);
            ignored.push(selectors.iter().flat_map(|s| s.all_rules()).collect());
        }
        Ok(Self {
            basenames: build(&basenames)?,
            absolutes: build(&absolutes)?,
            ignored,
        })
    }

    /// Effective rule set for `path`: `base` minus every matching glob's ignored rules.
    #[must_use]
    pub fn rules_for(&self, path: &Path, base: &RuleSet) -> RuleSet {
        let full = Candidate::new(path);
        let name = path.file_name().map(Candidate::new);

        // `matches_candidate` allocates; most files match nothing, so ask first.
        let by_name = name.filter(|name| self.basenames.is_match_candidate(name));
        let by_path = self.absolutes.is_match_candidate(&full);
        if by_name.is_none() && !by_path {
            return *base;
        }

        let mut rules = *base;
        let matches = by_name
            .map(|name| self.basenames.matches_candidate(&name))
            .unwrap_or_default()
            .into_iter()
            .chain(if by_path {
                self.absolutes.matches_candidate(&full)
            } else {
                vec![]
            });
        for idx in matches {
            for rule in &self.ignored[idx] {
                rules.remove(rule);
            }
        }
        rules
    }
}

/// Compile one glob, reporting `pattern` (the user's spelling) on failure.
fn compile(glob: &str, pattern: &str) -> Result<Glob> {
    Glob::new(glob)
        .map_err(|e| Error::Resolve(format!("Invalid per-file-ignores pattern '{pattern}': {e}")))
}

fn build(builder: &GlobSetBuilder) -> Result<GlobSet> {
    builder
        .build()
        .map_err(|e| Error::Resolve(format!("Failed to build per-file-ignores: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use djangofmt_lint::Rule;

    fn all_rules() -> RuleSet {
        RuleSelector::All.all_rules().collect()
    }

    fn ignores(patterns: &[(&str, Rule)]) -> PerFileIgnores {
        let mut map = BTreeMap::new();
        for (pattern, rule) in patterns {
            map.insert((*pattern).to_string(), vec![RuleSelector::Rule(*rule)]);
        }
        PerFileIgnores::new(&map, Path::new("/proj")).unwrap()
    }

    /// Worked example: a bare glob ignores by basename at any depth, a path glob is
    /// anchored at the root, ignores union across matches, and a rule listed for one
    /// glob is untouched on files that don't match it.
    #[test]
    fn per_file_ignores_worked_example() {
        let pfi = ignores(&[
            ("*.jinja", Rule::UseHttps),
            ("templates/admin/**", Rule::InvalidAttrValue),
        ]);
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

    /// `*` crosses `/`, as in ruff: `templates/*.html` covers nested files too.
    #[test]
    fn star_crosses_separators_like_ruff() {
        let pfi = ignores(&[("templates/*.html", Rule::UseHttps)]);
        let base = all_rules();
        for path in [
            "/proj/templates/page.html",
            "/proj/templates/admin/deep/page.html",
        ] {
            assert!(
                !pfi.rules_for(Path::new(path), &base).contains(Rule::UseHttps),
                "{path} should match `templates/*.html`"
            );
        }
    }

    #[test]
    fn invalid_pattern_errors() {
        let mut patterns = BTreeMap::new();
        patterns.insert("[unclosed".to_string(), vec![]);
        assert!(PerFileIgnores::new(&patterns, Path::new("/proj")).is_err());
    }
}

//! Fix applier and re-lint loop.
//!
//! [`apply_fixes`] runs a single forward pass over the source, applying
//! non-overlapping, non-isolation-conflicted fixes from the supplied
//! diagnostic list. [`lint_fix`] drives the multi-pass cascade (parse,
//! lint, apply, re-parse) up to [`MAX_FIX_ITERATIONS`].

use std::borrow::Cow;

use markup_fmt::SyntaxError;
use markup_fmt::ast::Root;
use markup_fmt::parser::Parser;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::LintDiagnostic;
use crate::Settings;
use crate::check_ast;
use crate::fix::{Applicability, IsolationLevel};

/// Metadata about a single applied fix, captured for `--show-fixes`.
#[derive(Debug, Clone)]
pub struct AppliedFix {
    /// Rule code of the diagnostic the fix came from.
    pub code: String,
    /// Short imperative summary of the fix, if the rule provided one.
    pub fix_title: Option<String>,
}

/// Result of a single fix-application pass.
#[derive(Debug)]
pub struct ApplyResult {
    /// The fixed source.
    pub output: String,
    /// Number of fixes applied.
    pub applied_count: usize,
    /// Number of fixes that were filtered out (overlap, isolation, applicability).
    pub skipped_count: usize,
    /// Source map mapping byte offsets in the original source to offsets in
    /// the output.
    pub source_map: SourceMap,
    /// Metadata for each applied fix, in application order.
    ///
    /// Used by the CLI's `--show-fixes` to render per-rule counts.
    pub applied_fixes: Vec<AppliedFix>,
}

/// Sequence of [`SourceMarker`]s describing the offset transformation from
/// the original source to the fixed output.
#[derive(Debug, Default)]
pub struct SourceMap {
    pub markers: Vec<SourceMarker>,
}

/// A start/end pair of byte offsets pinning a region in the original source
/// to its image in the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceMarker {
    /// Byte offset in the original source.
    pub source: usize,
    /// Byte offset in the output.
    pub dest: usize,
}

/// Maximum number of fix iterations before giving up.
pub const MAX_FIX_ITERATIONS: usize = 10;

/// Apply all fixes from `diagnostics` whose applicability is at least
/// `threshold` to `source`.
///
/// See the spec §5.1 for the algorithm. Strict `<` is used for overlap
/// detection (so two adjacent insertions at the same byte offset can both
/// apply).
#[must_use]
pub fn apply_fixes(
    source: &str,
    diagnostics: &[LintDiagnostic],
    threshold: Applicability,
) -> ApplyResult {
    let mut fixable: Vec<&LintDiagnostic> = diagnostics
        .iter()
        .filter(|d| d.fix.as_ref().is_some_and(|f| f.applies(threshold)))
        .collect();

    fixable.sort_by_key(|d| {
        d.fix
            .as_ref()
            .and_then(crate::fix::Fix::min_start)
            .unwrap_or(usize::MAX)
    });

    let mut output = String::with_capacity(source.len());
    let mut markers: Vec<SourceMarker> = Vec::new();
    let mut last_pos: usize = 0;
    let mut applied_count = 0usize;
    let mut skipped_count = 0usize;
    let mut applied_groups: FxHashSet<u32> = FxHashSet::default();
    let mut applied_fixes: Vec<AppliedFix> = Vec::new();

    for diag in fixable {
        let Some(fix) = diag.fix.as_ref() else {
            continue;
        };
        let edits = fix.edits();
        let Some(first) = edits.first() else {
            continue;
        };

        if let IsolationLevel::Group(id) = fix.isolation()
            && applied_groups.contains(&id)
        {
            skipped_count += 1;
            continue;
        }

        // Strict `<` so adjacent insertions at the same offset can both apply.
        if first.start() < last_pos {
            skipped_count += 1;
            continue;
        }

        for edit in edits {
            output.push_str(&source[last_pos..edit.start()]);
            markers.push(SourceMarker {
                source: edit.start(),
                dest: output.len(),
            });
            if let Some(content) = edit.content() {
                output.push_str(content);
            }
            markers.push(SourceMarker {
                source: edit.end(),
                dest: output.len(),
            });
            last_pos = edit.end();
        }

        if let IsolationLevel::Group(id) = fix.isolation() {
            applied_groups.insert(id);
        }
        applied_count += 1;
        applied_fixes.push(AppliedFix {
            code: diag.code.clone(),
            fix_title: diag.fix_title.clone(),
        });
    }

    output.push_str(&source[last_pos..]);

    ApplyResult {
        output,
        applied_count,
        skipped_count,
        source_map: SourceMap { markers },
        applied_fixes,
    }
}

/// Single-iteration helper: lint `ast` and apply the fixes in one pass.
#[must_use]
pub fn fix_ast(
    source: &str,
    ast: &Root<'_>,
    settings: &Settings,
    threshold: Applicability,
) -> ApplyResult {
    let diagnostics = check_ast(source, ast, settings);
    apply_fixes(source, &diagnostics, threshold)
}

/// Per-rule summary for `--show-fixes` output.
#[derive(Debug, Clone, Default)]
pub struct RuleFixSummary {
    /// Number of fixes applied for this rule across all iterations.
    pub count: usize,
    /// Short imperative summary of the fix, if the rule provided one.
    pub fix_title: Option<String>,
}

/// Result of [`lint_fix`].
#[derive(Debug)]
pub struct FixerResult {
    /// The (possibly fixed) source.
    pub source: String,
    /// Diagnostics remaining after the final iteration.
    pub remaining_diagnostics: Vec<LintDiagnostic>,
    /// Total fixes applied across iterations.
    pub applied_count: usize,
    /// Total fixes skipped (overlap, isolation, applicability).
    pub skipped_count: usize,
    /// Number of fix iterations that ran.
    pub iterations: usize,
    /// Per-rule applied summaries, used by `--show-fixes`.
    pub applied_by_rule: rustc_hash::FxHashMap<String, RuleFixSummary>,
}

/// Errors from [`lint_fix`].
#[derive(Debug)]
pub enum FixerError {
    /// A fix introduced a syntax error in source that previously parsed.
    SyntaxRegression {
        iteration: usize,
        error: SyntaxError,
    },
}

/// Run the lint-and-fix loop on `source`.
///
/// 1. Parse → lint → apply.
/// 2. If anything was applied, re-parse and repeat up to
///    [`MAX_FIX_ITERATIONS`].
/// 3. Bail with [`FixerError::SyntaxRegression`] if a fix introduced a
///    syntax error in source that previously parsed.
/// 4. On convergence-failure (iteration limit reached with more fixes
///    pending), log a warning and return the work done so far.
pub fn lint_fix(
    source: &str,
    settings: &Settings,
    profile: markup_fmt::Language,
    threshold: Applicability,
) -> Result<FixerResult, FixerError> {
    let mut current: Cow<'_, str> = Cow::Borrowed(source);
    let mut total_applied = 0usize;
    let mut total_skipped = 0usize;
    let mut iterations = 0usize;
    let mut had_valid_first_parse = false;
    let mut applied_by_rule: FxHashMap<String, RuleFixSummary> = FxHashMap::default();

    loop {
        if iterations >= MAX_FIX_ITERATIONS {
            tracing::warn!(
                "Fix iteration limit reached; remaining diagnostics may be fixable on a second run."
            );
            return Ok(FixerResult {
                source: current.into_owned(),
                // Diagnostics from the previous iteration are span-aligned to
                // the pre-apply source, not to `current`; rather than return
                // mismatched spans, drop them. A re-run will re-derive them.
                remaining_diagnostics: Vec::new(),
                applied_count: total_applied,
                skipped_count: total_skipped,
                iterations,
                applied_by_rule,
            });
        }

        let mut parser = Parser::new(&current, profile, vec![]);
        let ast = match parser.parse_root() {
            Ok(ast) => {
                if iterations == 0 {
                    had_valid_first_parse = true;
                }
                ast
            }
            Err(err) if had_valid_first_parse => {
                return Err(FixerError::SyntaxRegression {
                    iteration: iterations,
                    error: err,
                });
            }
            Err(_) => {
                return Ok(FixerResult {
                    source: current.into_owned(),
                    remaining_diagnostics: Vec::new(),
                    applied_count: total_applied,
                    skipped_count: total_skipped,
                    iterations,
                    applied_by_rule,
                });
            }
        };

        let diagnostics = check_ast(&current, &ast, settings);
        let result = apply_fixes(&current, &diagnostics, threshold);
        total_skipped += result.skipped_count;
        for applied in &result.applied_fixes {
            let entry = applied_by_rule.entry(applied.code.clone()).or_default();
            entry.count += 1;
            if entry.fix_title.is_none() {
                entry.fix_title.clone_from(&applied.fix_title);
            }
        }

        if result.applied_count == 0 {
            return Ok(FixerResult {
                source: current.into_owned(),
                remaining_diagnostics: diagnostics,
                applied_count: total_applied,
                skipped_count: total_skipped,
                iterations,
                applied_by_rule,
            });
        }

        total_applied += result.applied_count;
        current = Cow::Owned(result.output);
        iterations += 1;
    }
}

#[cfg(test)]
mod tests {
    use miette::SourceSpan;

    use super::*;
    use crate::fix::{Edit, Fix};

    fn span(start: usize, len: usize) -> SourceSpan {
        SourceSpan::new(start.into(), len)
    }

    fn diag_with_fix(fix: Fix) -> LintDiagnostic {
        LintDiagnostic {
            code: "test-rule".to_string(),
            message: "test".to_string(),
            span: span(0, 0),
            help: None,
            fix: Some(fix),
            fix_title: None,
        }
    }

    fn diag_without_fix() -> LintDiagnostic {
        LintDiagnostic {
            code: "test-rule".to_string(),
            message: "test".to_string(),
            span: span(0, 0),
            help: None,
            fix: None,
            fix_title: None,
        }
    }

    #[test]
    fn single_replacement() {
        let source = "hello world";
        let diags = vec![diag_with_fix(Fix::safe_edit(Edit::replacement(
            "Rust",
            span(6, 5),
        )))];
        let result = apply_fixes(source, &diags, Applicability::Safe);
        assert_eq!(result.output, "hello Rust");
        assert_eq!(result.applied_count, 1);
        assert_eq!(result.skipped_count, 0);
    }

    #[test]
    fn multiple_non_overlapping() {
        let source = "abcdefghij";
        let diags = vec![
            diag_with_fix(Fix::safe_edit(Edit::replacement("X", span(0, 1)))),
            diag_with_fix(Fix::safe_edit(Edit::replacement("Y", span(5, 1)))),
        ];
        let result = apply_fixes(source, &diags, Applicability::Safe);
        assert_eq!(result.output, "XbcdeYghij");
        assert_eq!(result.applied_count, 2);
    }

    #[test]
    fn overlap_rejected() {
        let source = "abcdefghij";
        let diags = vec![
            diag_with_fix(Fix::safe_edit(Edit::replacement("XX", span(0, 5)))),
            diag_with_fix(Fix::safe_edit(Edit::replacement("YY", span(2, 5)))),
        ];
        let result = apply_fixes(source, &diags, Applicability::Safe);
        // First one applied, second overlaps and is skipped.
        assert_eq!(result.output, "XXfghij");
        assert_eq!(result.applied_count, 1);
        assert_eq!(result.skipped_count, 1);
    }

    #[test]
    fn isolation_groups() {
        let source = "abcdefghij";
        let diags = vec![
            diag_with_fix(
                Fix::safe_edit(Edit::replacement("X", span(0, 1)))
                    .isolate(IsolationLevel::Group(1)),
            ),
            diag_with_fix(
                Fix::safe_edit(Edit::replacement("Y", span(5, 1)))
                    .isolate(IsolationLevel::Group(1)),
            ),
        ];
        let result = apply_fixes(source, &diags, Applicability::Safe);
        // Only the first fix in group 1 applies.
        assert_eq!(result.output, "Xbcdefghij");
        assert_eq!(result.applied_count, 1);
        assert_eq!(result.skipped_count, 1);
    }

    #[test]
    fn applicability_filtering() {
        let source = "abcdefghij";
        let diags = vec![
            diag_with_fix(Fix::safe_edit(Edit::replacement("S", span(0, 1)))),
            diag_with_fix(Fix::unsafe_edit(Edit::replacement("U", span(2, 1)))),
            diag_with_fix(Fix::display_only_edit(Edit::replacement("D", span(4, 1)))),
        ];

        // Safe threshold: only the safe fix applies.
        let result = apply_fixes(source, &diags, Applicability::Safe);
        assert_eq!(result.output, "Sbcdefghij");
        assert_eq!(result.applied_count, 1);

        // Unsafe threshold: safe + unsafe apply.
        let result = apply_fixes(source, &diags, Applicability::Unsafe);
        assert_eq!(result.output, "SbUdefghij");
        assert_eq!(result.applied_count, 2);

        // DisplayOnly threshold: all apply.
        let result = apply_fixes(source, &diags, Applicability::DisplayOnly);
        assert_eq!(result.output, "SbUdDfghij");
        assert_eq!(result.applied_count, 3);
    }

    #[test]
    fn deletion() {
        let source = "abcdefghij";
        let diags = vec![diag_with_fix(Fix::safe_edit(Edit::deletion(span(2, 3))))];
        let result = apply_fixes(source, &diags, Applicability::Safe);
        assert_eq!(result.output, "abfghij");
        assert_eq!(result.applied_count, 1);
    }

    #[test]
    fn insertion_at_same_offset() {
        // Two adjacent insertions at the same byte offset can both apply
        // (strict `<` overlap semantics).
        let source = "abc";
        let diags = vec![
            diag_with_fix(Fix::safe_edit(Edit::insertion("X", 1))),
            diag_with_fix(Fix::safe_edit(Edit::insertion("Y", 1))),
        ];
        let result = apply_fixes(source, &diags, Applicability::Safe);
        assert_eq!(result.output, "aXYbc");
        assert_eq!(result.applied_count, 2);
    }

    #[test]
    fn ignores_diagnostics_without_fix() {
        let source = "abc";
        let diags = vec![
            diag_without_fix(),
            diag_with_fix(Fix::safe_edit(Edit::replacement("X", span(0, 1)))),
        ];
        let result = apply_fixes(source, &diags, Applicability::Safe);
        assert_eq!(result.output, "Xbc");
        assert_eq!(result.applied_count, 1);
    }

    #[test]
    fn empty_diagnostics() {
        let source = "abc";
        let result = apply_fixes(source, &[], Applicability::Safe);
        assert_eq!(result.output, "abc");
        assert_eq!(result.applied_count, 0);
        assert_eq!(result.skipped_count, 0);
    }

    #[test]
    fn source_map_markers() {
        let source = "hello world";
        let diags = vec![diag_with_fix(Fix::safe_edit(Edit::replacement(
            "Rust",
            span(6, 5),
        )))];
        let result = apply_fixes(source, &diags, Applicability::Safe);
        assert_eq!(result.source_map.markers.len(), 2);
        assert_eq!(
            result.source_map.markers[0],
            SourceMarker { source: 6, dest: 6 }
        );
        assert_eq!(
            result.source_map.markers[1],
            SourceMarker {
                source: 11,
                dest: 10
            }
        );
    }

    #[test]
    fn lint_fix_no_op_returns_no_iterations() {
        let source = "<div></div>";
        let settings = Settings::all();
        let result = lint_fix(
            source,
            &settings,
            markup_fmt::Language::Django,
            Applicability::Safe,
        )
        .expect("lint_fix");
        assert_eq!(result.source, source);
        assert_eq!(result.applied_count, 0);
        assert_eq!(result.iterations, 0);
    }
}

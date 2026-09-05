//! [`LintContext`] and [`DiagnosticGuard`].
//!
//! The `LintContext` owns the diagnostic buffer and is shared by reference through the AST visitor.
//! Reporting a diagnostic returns a guard that buffers a partial diagnostic; on Drop the guard
//! pushes it into the context's buffer.

use std::cell::RefCell;
use std::path::Path;

use miette::SourceSpan;

use crate::LintDiagnostic;
use crate::Settings;
use crate::fix::Fix;
use crate::registry::Rule;
use crate::rule_set::RuleSet;
use crate::span;
use crate::suppression;
use crate::violation::Violation;

/// A type for collecting diagnostics in a given file.
///
/// [`LintContext::report_diagnostic`] can be used to obtain a [`DiagnosticGuard`], which will push
/// a [`Violation`] to the contained [`Diagnostic`] collection on `Drop`.
pub struct LintContext<'a> {
    diagnostics: RefCell<Vec<LintDiagnostic>>,
    source: &'a str,
    settings: &'a Settings,
    path: Option<&'a Path>,
    /// The run's rules minus the file's own `file-ignore[...]` opt-outs.
    rules: RuleSet,
}

impl<'a> LintContext<'a> {
    #[must_use]
    pub fn new(source: &'a str, settings: &'a Settings, path: Option<&'a Path>) -> Self {
        let mut rules = settings.rules;
        rules.remove_all(&suppression::file_ignored_rules(source));
        Self {
            source,
            settings,
            path,
            rules,
            diagnostics: RefCell::new(Vec::new()),
        }
    }

    /// The source code being linted.
    #[must_use]
    pub const fn source(&self) -> &'a str {
        self.source
    }

    /// The path of the file being linted, or [`None`] when there is no backing file.
    #[must_use]
    pub const fn path(&self) -> Option<&'a Path> {
        self.path
    }

    /// The settings active for this run.
    #[must_use]
    pub const fn settings(&self) -> &'a Settings {
        self.settings
    }

    /// Returns whether the given rule should be checked.
    #[must_use]
    #[inline]
    pub const fn is_rule_enabled(&self, rule: Rule) -> bool {
        self.rules.contains(rule)
    }

    /// Returns whether any of the given rules should be checked.
    #[must_use]
    #[inline]
    pub const fn any_rule_enabled(&self, rules: &[Rule]) -> bool {
        self.rules.contains_any(rules)
    }

    /// Compute the byte offset of `slice` within the source.
    ///
    /// # Panics
    ///
    /// Panics if `slice` is not a subslice of the source.
    #[must_use]
    pub fn source_offset(&self, slice: &str) -> usize {
        let src_start = self.source.as_ptr() as usize;
        let src_end = src_start + self.source.len();
        let slice_start = slice.as_ptr() as usize;
        let slice_end = slice_start + slice.len();

        assert!(
            slice_start >= src_start && slice_end <= src_end,
            "slice must be a subslice of self.source"
        );

        slice_start - src_start
    }

    /// The byte offset just past `slice` within the source.
    ///
    /// # Panics
    ///
    /// Panics if `slice` is not a subslice of the source.
    #[must_use]
    pub fn source_end(&self, slice: &str) -> usize {
        self.source_offset(slice) + slice.len()
    }

    /// The span of `slice` within the source.
    ///
    /// # Panics
    ///
    /// Panics if `slice` is not a subslice of the source.
    #[must_use]
    pub fn source_span(&self, slice: &str) -> SourceSpan {
        span(self.source_offset(slice), slice.len())
    }

    /// Build a guard for an enabled rule. Returns `None` if the rule is disabled.
    pub fn report_diagnostic_if_enabled<V: Violation>(
        &self,
        violation: &V,
        span: SourceSpan,
    ) -> Option<DiagnosticGuard<'_, 'a>> {
        if !self.is_rule_enabled(V::RULE) {
            return None;
        }
        Some(self.report_diagnostic(violation, span))
    }

    /// Build a guard for a rule the caller has already gated with
    /// [`Self::is_rule_enabled`].
    pub fn report_diagnostic<V: Violation>(
        &self,
        violation: &V,
        span: SourceSpan,
    ) -> DiagnosticGuard<'_, 'a> {
        DiagnosticGuard {
            context: self,
            diagnostic: Some(LintDiagnostic {
                code: V::RULE.into(),
                message: violation.message(),
                span,
                help: violation.help(),
                fix: None,
                fix_title: violation.fix_title(),
            }),
        }
    }

    /// Drop every diagnostic `keep` rejects: suppression applies once all rules have run.
    pub fn retain_diagnostics(&self, keep: impl FnMut(&LintDiagnostic) -> bool) {
        self.diagnostics.borrow_mut().retain(keep);
    }

    /// Consume the context and return the collected diagnostics.
    #[must_use]
    pub fn into_diagnostics(self) -> Vec<LintDiagnostic> {
        self.diagnostics.into_inner()
    }
}

/// Guard that holds a partially-built diagnostic and pushes it on Drop.
pub struct DiagnosticGuard<'ctx, 'a> {
    context: &'ctx LintContext<'a>,
    diagnostic: Option<LintDiagnostic>,
}

impl DiagnosticGuard<'_, '_> {
    /// Attach a fix. Stored on the diagnostic; applicability gating happens
    /// at apply time.
    pub fn set_fix(&mut self, fix: Fix) {
        if let Some(diag) = self.diagnostic.as_mut() {
            diag.fix = Some(fix);
        }
    }
}

impl Drop for DiagnosticGuard<'_, '_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            // Don't push half-built diagnostics if we're unwinding.
            return;
        }
        if let Some(diag) = self.diagnostic.take() {
            self.context.diagnostics.borrow_mut().push(diag);
        }
    }
}

//! Fix data model.
//!
//! a [`Fix`] is one or more non-overlapping [`Edit`]s tagged with an [`Applicability`] threshold and an [`IsolationLevel`].
//!
//! Rules attach a [`Fix`] to a [`crate::LintDiagnostic`] via the diagnostic guard
//! returned from [`crate::LintContext::report_diagnostic_if_enabled`]. The applier
//! ([`apply::apply_fixes`]) gates by applicability, sorts by start position, and
//! resolves overlap conflicts in a single forward pass.

pub mod apply;

use std::cmp::Ordering;

use miette::SourceSpan;

/// Forward-looking declaration of fix availability for a rule.
///
/// Carried on the [`crate::violation::Violation`] trait so rule authors declare
/// fix availability upfront.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixAvailability {
    Sometimes,
    Always,
    None,
}

impl FixAvailability {
    /// Short capitalized label suitable for a docs table cell.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Always => "Always",
            Self::Sometimes => "Sometimes",
            Self::None => "None",
        }
    }
}

impl std::fmt::Display for FixAvailability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Always => write!(f, "Fix is always available."),
            Self::Sometimes => write!(f, "Fix is sometimes available."),
            Self::None => write!(f, "Fix is not available."),
        }
    }
}

/// Three-tier safety classification for fixes, ordered ascending so `>=`
/// reads as a "minimum required" applicability check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Applicability {
    /// Likely incorrect; surfaced in diagnostics but never applied automatically.
    DisplayOnly,
    /// May change runtime behavior; applied only with `--unsafe-fixes`.
    Unsafe,
    /// Preserves exact semantics; applied with `--fix`.
    Safe,
}

/// Isolation level for a [`Fix`]. Used when two rules can produce mutually
/// exclusive fixes (e.g. "remove attr X" vs "rewrite attr X to Y").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IsolationLevel {
    #[default]
    NonOverlapping,
    /// Only one fix in this group is applied per pass; first-wins by sort order.
    Group(u32),
}

/// A single text mutation.
///
/// `range` is a [`miette::SourceSpan`] for codebase consistency.
///
/// `content` uses [`Option<Box<str>>`] to distinguish pure deletion ([`None`])
/// from non-empty replacement ([`Some(...)`]) while preserving the niche
/// optimization (same size as `Box<str>`). Empty replacement content is
/// rejected by [`Edit::replacement`] and [`Edit::insertion`] via a
/// `debug_assert!`; use [`Edit::deletion`] for the empty case.
#[derive(Clone, Debug)]
pub struct Edit {
    range: SourceSpan,
    content: Option<Box<str>>,
}

impl Edit {
    /// Replace `range` with `content`.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if `content` is empty. Use [`Edit::deletion`]
    /// for pure deletions.
    #[must_use]
    pub fn replacement(content: impl Into<Box<str>>, range: SourceSpan) -> Self {
        let content = content.into();
        debug_assert!(
            !content.is_empty(),
            "use `Edit::deletion` for empty replacement content"
        );
        Self {
            range,
            content: Some(content),
        }
    }

    /// Insert `content` at byte offset `at`.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if `content` is empty.
    #[must_use]
    pub fn insertion(content: impl Into<Box<str>>, at: usize) -> Self {
        let content = content.into();
        debug_assert!(
            !content.is_empty(),
            "use `Edit::deletion` for empty insertion content"
        );
        Self {
            range: SourceSpan::new(at.into(), 0),
            content: Some(content),
        }
    }

    /// Delete the bytes in `range`.
    #[must_use]
    pub const fn deletion(range: SourceSpan) -> Self {
        Self {
            range,
            content: None,
        }
    }

    /// Start offset (inclusive).
    #[must_use]
    pub const fn start(&self) -> usize {
        self.range.offset()
    }

    /// End offset (exclusive).
    #[must_use]
    pub const fn end(&self) -> usize {
        self.range.offset() + self.range.len()
    }

    /// Replacement / insertion content, or [`None`] for pure deletion.
    #[must_use]
    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }
}

impl PartialEq for Edit {
    fn eq(&self, other: &Self) -> bool {
        self.start() == other.start() && self.end() == other.end() && self.content == other.content
    }
}

impl Eq for Edit {}

impl Ord for Edit {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start()
            .cmp(&other.start())
            .then_with(|| self.end().cmp(&other.end()))
            .then_with(|| self.content.cmp(&other.content))
    }
}

impl PartialOrd for Edit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A grouped set of [`Edit`]s applied atomically.
///
/// Multi-edit constructors sort edits by `(start, end)` and **debug-assert**
/// that no two edits within a single [`Fix`] overlap. This is a precondition:
/// violations within a fix indicate a rule bug.
#[derive(Debug, Clone)]
pub struct Fix {
    edits: Vec<Edit>,
    applicability: Applicability,
    isolation_level: IsolationLevel,
}

impl Fix {
    fn single(edit: Edit, applicability: Applicability) -> Self {
        Self {
            edits: vec![edit],
            applicability,
            isolation_level: IsolationLevel::default(),
        }
    }

    fn multi<I>(first: Edit, rest: I, applicability: Applicability) -> Self
    where
        I: IntoIterator<Item = Edit>,
    {
        let mut edits: Vec<Edit> = std::iter::once(first).chain(rest).collect();
        edits.sort();

        debug_assert!(
            edits
                .windows(2)
                .all(|window| window[0].end() <= window[1].start()),
            "edits within a `Fix` must be non-overlapping"
        );

        Self {
            edits,
            applicability,
            isolation_level: IsolationLevel::default(),
        }
    }

    /// Single-edit safe fix.
    #[must_use]
    pub fn safe_edit(edit: Edit) -> Self {
        Self::single(edit, Applicability::Safe)
    }

    /// Single-edit unsafe fix.
    #[must_use]
    pub fn unsafe_edit(edit: Edit) -> Self {
        Self::single(edit, Applicability::Unsafe)
    }

    /// Single-edit display-only fix.
    #[must_use]
    pub fn display_only_edit(edit: Edit) -> Self {
        Self::single(edit, Applicability::DisplayOnly)
    }

    /// Multi-edit safe fix.
    #[must_use]
    pub fn safe_edits<I>(first: Edit, rest: I) -> Self
    where
        I: IntoIterator<Item = Edit>,
    {
        Self::multi(first, rest, Applicability::Safe)
    }

    /// Multi-edit unsafe fix.
    #[must_use]
    pub fn unsafe_edits<I>(first: Edit, rest: I) -> Self
    where
        I: IntoIterator<Item = Edit>,
    {
        Self::multi(first, rest, Applicability::Unsafe)
    }

    /// Multi-edit display-only fix.
    #[must_use]
    pub fn display_only_edits<I>(first: Edit, rest: I) -> Self
    where
        I: IntoIterator<Item = Edit>,
    {
        Self::multi(first, rest, Applicability::DisplayOnly)
    }

    /// Set the isolation level (builder).
    #[must_use]
    pub const fn isolate(mut self, level: IsolationLevel) -> Self {
        self.isolation_level = level;
        self
    }

    /// Override the applicability (builder).
    #[must_use]
    pub const fn with_applicability(mut self, applicability: Applicability) -> Self {
        self.applicability = applicability;
        self
    }

    #[must_use]
    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }

    #[must_use]
    pub const fn applicability(&self) -> Applicability {
        self.applicability
    }

    #[must_use]
    pub const fn isolation(&self) -> IsolationLevel {
        self.isolation_level
    }

    /// First edit's start offset, used for sorting.
    #[must_use]
    pub fn min_start(&self) -> Option<usize> {
        self.edits.first().map(Edit::start)
    }

    /// True iff this fix's applicability meets `threshold`.
    #[must_use]
    pub fn applies(&self, threshold: Applicability) -> bool {
        self.applicability >= threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: usize, len: usize) -> SourceSpan {
        SourceSpan::new(start.into(), len)
    }

    #[test]
    fn edit_replacement() {
        let edit = Edit::replacement("hello", span(0, 5));
        assert_eq!(edit.start(), 0);
        assert_eq!(edit.end(), 5);
        assert_eq!(edit.content(), Some("hello"));
    }

    #[test]
    fn edit_insertion() {
        let edit = Edit::insertion("xyz", 4);
        assert_eq!(edit.start(), 4);
        assert_eq!(edit.end(), 4);
        assert_eq!(edit.content(), Some("xyz"));
    }

    #[test]
    fn edit_deletion() {
        let edit = Edit::deletion(span(2, 3));
        assert_eq!(edit.start(), 2);
        assert_eq!(edit.end(), 5);
        assert_eq!(edit.content(), None);
    }

    #[test]
    #[should_panic(expected = "Edit::deletion")]
    fn edit_replacement_empty_panics_in_debug() {
        let _ = Edit::replacement("", span(0, 0));
    }

    #[test]
    #[should_panic(expected = "Edit::deletion")]
    fn edit_insertion_empty_panics_in_debug() {
        let _ = Edit::insertion("", 0);
    }

    #[test]
    fn edit_ord_by_start_then_end_then_content() {
        let mut edits = [
            Edit::replacement("c", span(2, 1)),
            Edit::replacement("a", span(0, 1)),
            Edit::replacement("b", span(0, 1)),
            Edit::deletion(span(2, 2)),
        ];
        edits.sort();
        assert_eq!(edits[0].content(), Some("a"));
        assert_eq!(edits[1].content(), Some("b"));
        assert_eq!(edits[2].start(), 2);
        assert_eq!(edits[2].end(), 3);
        assert_eq!(edits[3].start(), 2);
        assert_eq!(edits[3].end(), 4);
    }

    #[test]
    fn fix_safe_edit() {
        let fix = Fix::safe_edit(Edit::insertion("x", 5));
        assert_eq!(fix.applicability(), Applicability::Safe);
        assert_eq!(fix.edits().len(), 1);
        assert_eq!(fix.min_start(), Some(5));
        assert_eq!(fix.isolation(), IsolationLevel::NonOverlapping);
    }

    #[test]
    fn fix_unsafe_edit() {
        let fix = Fix::unsafe_edit(Edit::insertion("x", 5));
        assert_eq!(fix.applicability(), Applicability::Unsafe);
    }

    #[test]
    fn fix_display_only_edit() {
        let fix = Fix::display_only_edit(Edit::insertion("x", 5));
        assert_eq!(fix.applicability(), Applicability::DisplayOnly);
    }

    #[test]
    fn fix_safe_edits_sorts() {
        let fix = Fix::safe_edits(
            Edit::insertion("late", 10),
            [Edit::insertion("early", 0), Edit::insertion("mid", 5)],
        );
        let starts: Vec<usize> = fix.edits().iter().map(Edit::start).collect();
        assert_eq!(starts, vec![0, 5, 10]);
        assert_eq!(fix.min_start(), Some(0));
    }

    #[test]
    #[should_panic(expected = "non-overlapping")]
    fn fix_safe_edits_overlap_panics_in_debug() {
        let _ = Fix::safe_edits(
            Edit::replacement("a", span(0, 5)),
            [Edit::replacement("b", span(3, 5))],
        );
    }

    #[test]
    fn fix_unsafe_edits() {
        let fix = Fix::unsafe_edits(Edit::insertion("a", 0), [Edit::insertion("b", 5)]);
        assert_eq!(fix.applicability(), Applicability::Unsafe);
        assert_eq!(fix.edits().len(), 2);
    }

    #[test]
    fn fix_display_only_edits() {
        let fix = Fix::display_only_edits(Edit::insertion("a", 0), [Edit::insertion("b", 5)]);
        assert_eq!(fix.applicability(), Applicability::DisplayOnly);
    }

    #[test]
    fn fix_isolate_builder() {
        let fix = Fix::safe_edit(Edit::insertion("x", 0)).isolate(IsolationLevel::Group(7));
        assert!(matches!(fix.isolation(), IsolationLevel::Group(7)));
    }

    #[test]
    fn fix_with_applicability_builder() {
        let fix = Fix::safe_edit(Edit::insertion("x", 0)).with_applicability(Applicability::Unsafe);
        assert_eq!(fix.applicability(), Applicability::Unsafe);
    }

    #[test]
    fn fix_applies_threshold_semantics() {
        let safe = Fix::safe_edit(Edit::insertion("x", 0));
        let unsafe_ = Fix::unsafe_edit(Edit::insertion("x", 0));
        let display = Fix::display_only_edit(Edit::insertion("x", 0));

        // threshold = Safe admits only Safe
        assert!(safe.applies(Applicability::Safe));
        assert!(!unsafe_.applies(Applicability::Safe));
        assert!(!display.applies(Applicability::Safe));

        // threshold = Unsafe admits Safe + Unsafe
        assert!(safe.applies(Applicability::Unsafe));
        assert!(unsafe_.applies(Applicability::Unsafe));
        assert!(!display.applies(Applicability::Unsafe));

        // threshold = DisplayOnly admits everything
        assert!(safe.applies(Applicability::DisplayOnly));
        assert!(unsafe_.applies(Applicability::DisplayOnly));
        assert!(display.applies(Applicability::DisplayOnly));
    }

    #[test]
    fn fix_safe_edits_adjacent_allowed() {
        // Adjacent edits (end of one == start of next) are allowed.
        let fix = Fix::safe_edits(
            Edit::replacement("a", span(0, 3)),
            [Edit::replacement("b", span(3, 2))],
        );
        assert_eq!(fix.edits().len(), 2);
    }
}

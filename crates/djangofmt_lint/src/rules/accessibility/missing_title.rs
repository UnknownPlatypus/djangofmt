use std::borrow::Cow;

use markup_fmt::ast::{Element, JinjaTagOrChildren, Node, NodeKind};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

#[derive(Debug, PartialEq, Eq)]
pub enum TitleViolation {
    /// No `<title>` element is present anywhere inside `<head>`.
    Absent,
    /// A `<title>` element exists but has no visible or templated content.
    Empty,
    /// The `<html>` document has no `<head>` element at all.
    NoHead,
}

/// ## What it does
/// Checks for `<head>` elements that do not contain a non-empty `<title>` child, and for `<html>`
/// documents with no `<head>` at all.
///
/// ## Why is this bad?
/// The `<title>` element names the document. Browsers display it in tabs, history entries, and
/// bookmarks; screen readers announce it first when the page loads; and search engines use it as
/// the default link text in result pages. A page without a title leaves users unable to tell tabs
/// apart and fails WCAG Success Criterion 2.4.2.
///
/// ## Example
/// ```html
/// <head>
///     <meta charset="utf-8">
/// </head>
/// ```
///
/// Use instead:
/// ```html
/// <head>
///     <meta charset="utf-8">
///     <title>My page</title>
/// </head>
/// ```
///
/// ## References
/// - [WCAG 2.4.2: Page Titled](https://www.w3.org/WAI/WCAG21/Understanding/page-titled.html)
/// - [HTML spec: the title element](https://html.spec.whatwg.org/multipage/semantics.html#the-title-element)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.9")]
pub struct MissingTitle {
    pub kind: TitleViolation,
}

impl Violation for MissingTitle {
    const RULE: Rule = Rule::MissingTitle;
    const CATEGORY: RuleCategory = RuleCategory::Accessibility;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        match self.kind {
            TitleViolation::Absent | TitleViolation::Empty => {
                "Missing or empty `<title>` in `<head>`".into()
            }
            TitleViolation::NoHead => "Missing `<head>` and `<title>` in `<html>`".into(),
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some(match self.kind {
            TitleViolation::Absent => "Add a `<title>` element to `<head>`".into(),
            TitleViolation::Empty => "Add descriptive text to `<title>`".into(),
            TitleViolation::NoHead => "Add a `<head>` containing a `<title>` element".into(),
        })
    }
}

/// The caller guarantees `element` is a `<head>`.
pub fn check(checker: &Checker<'_>, element: &Element<'_>) {
    let status = classify_title(&element.children, &node_title_status);
    let Some(kind) = status.violation(TitleViolation::Absent) else {
        return;
    };

    checker.report_diagnostic(
        &MissingTitle { kind },
        checker.source_span(element.tag_name),
    );
}

/// The caller guarantees `element` is an `<html>`.
pub fn check_html(checker: &Checker<'_>, element: &Element<'_>) {
    let status = classify_title(&element.children, &root_node_title_status);
    let Some(kind) = status.violation(TitleViolation::NoHead) else {
        return;
    };

    checker.report_diagnostic(
        &MissingTitle { kind },
        checker.source_span(element.tag_name),
    );
}

/// Outcome of inspecting a node list for the `<title>` it carries.
enum TitleStatus {
    /// A `<head>`, or a template tag that may render one, carries the title.
    Deferred,
    /// A non-empty `<title>` was found.
    Present,
    /// At least one `<title>` was found, but none had content.
    Empty,
    /// No `<title>` element was found.
    Absent,
}

impl TitleStatus {
    const fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Deferred, _) | (_, Self::Deferred) => Self::Deferred,
            (Self::Present, _) | (_, Self::Present) => Self::Present,
            (Self::Empty, _) | (_, Self::Empty) => Self::Empty,
            _ => Self::Absent,
        }
    }

    /// The violation to report, or `None` when a titled document was found. Callers pass the
    /// kind standing for "no `<title>` here", which differs between a `<head>` and an `<html>`.
    const fn violation(self, absent: TitleViolation) -> Option<TitleViolation> {
        match self {
            Self::Deferred | Self::Present => None,
            Self::Empty => Some(TitleViolation::Empty),
            Self::Absent => Some(absent),
        }
    }
}

/// Classify a node list by folding `node_status` over it, descending into Jinja block branches.
fn classify_title(
    nodes: &[Node<'_>],
    node_status: &impl Fn(&Node<'_>) -> TitleStatus,
) -> TitleStatus {
    nodes.iter().fold(TitleStatus::Absent, |acc, node| {
        let status = match &node.kind {
            NodeKind::JinjaBlock(block) => {
                block
                    .body
                    .iter()
                    .fold(TitleStatus::Absent, |acc, item| match item {
                        JinjaTagOrChildren::Children(children) => {
                            acc.merge(classify_title(children, node_status))
                        }
                        JinjaTagOrChildren::Tag(_) => acc,
                    })
            }
            _ => node_status(node),
        };
        acc.merge(status)
    })
}

/// Inside a `<head>`, only a `<title>` carries the title.
fn node_title_status(node: &Node<'_>) -> TitleStatus {
    match &node.kind {
        NodeKind::Element(el) if el.tag_name.eq_ignore_ascii_case("title") => {
            if title_has_content(&el.children) {
                TitleStatus::Present
            } else {
                TitleStatus::Empty
            }
        }
        _ => TitleStatus::Absent,
    }
}

/// Directly under `<html>`, the `<head>` start tag is optional, so a bare `<title>` counts. A
/// `<head>`, or a template tag that may render one, hands the title to the `<head>` check.
fn root_node_title_status(node: &Node<'_>) -> TitleStatus {
    match &node.kind {
        NodeKind::Element(el) if el.tag_name.eq_ignore_ascii_case("head") => TitleStatus::Deferred,
        NodeKind::JinjaTag(_) => TitleStatus::Deferred,
        _ => node_title_status(node),
    }
}

/// Whether `<title>`'s children carry visible or templated content.
///
/// Whitespace-only text is treated as empty; Jinja interpolations, tags, and blocks count as
/// content because they may expand to text at render time.
fn title_has_content(children: &[Node<'_>]) -> bool {
    children.iter().any(|node| match &node.kind {
        NodeKind::Text(text) => !text.raw.trim().is_empty(),
        NodeKind::JinjaInterpolation(_) | NodeKind::JinjaTag(_) | NodeKind::Element(_) => true,
        NodeKind::JinjaBlock(block) => block.body.iter().any(|item| match item {
            JinjaTagOrChildren::Children(children) => title_has_content(children),
            JinjaTagOrChildren::Tag(_) => true,
        }),
        _ => false,
    })
}

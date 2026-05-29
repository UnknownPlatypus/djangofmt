use markup_fmt::ast::{Element, JinjaBlock, JinjaTagOrChildren, Node, NodeKind};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

#[derive(Debug, PartialEq, Eq)]
pub enum TitleViolation {
    /// No `<title>` element is present anywhere inside `<head>`.
    Absent,
    /// A `<title>` element exists but has no visible or templated content.
    Empty,
}

/// ## What it does
/// Checks for `<head>` elements that do not contain a non-empty `<title>` child.
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
    fn message(&self) -> String {
        "Missing or empty `<title>` in `<head>`.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some(match self.kind {
            TitleViolation::Absent => {
                "Add a `<title>` element with descriptive text inside `<head>`.".to_string()
            }
            TitleViolation::Empty => {
                "Fill the existing `<title>` element with descriptive text.".to_string()
            }
        })
    }
}

pub fn check(element: &Element<'_>, checker: &Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("head") {
        return;
    }

    let kind = match classify_title(&element.children) {
        TitleStatus::Present => return,
        TitleStatus::Empty => TitleViolation::Empty,
        TitleStatus::Absent => TitleViolation::Absent,
    };

    let offset = checker.source_offset(element.tag_name);
    checker.report_diagnostic(
        &MissingTitle { kind },
        (offset, element.tag_name.len()).into(),
    );
}

/// Outcome of inspecting a `<head>`'s descendants for a `<title>`.
enum TitleStatus {
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
            (Self::Present, _) | (_, Self::Present) => Self::Present,
            (Self::Empty, _) | (_, Self::Empty) => Self::Empty,
            _ => Self::Absent,
        }
    }
}

/// Classify the title situation across a node list (head children or Jinja block children).
fn classify_title(nodes: &[Node<'_>]) -> TitleStatus {
    nodes.iter().fold(TitleStatus::Absent, |acc, node| {
        acc.merge(node_title_status(node))
    })
}

fn node_title_status(node: &Node<'_>) -> TitleStatus {
    match &node.kind {
        NodeKind::Element(el) if el.tag_name.eq_ignore_ascii_case("title") => {
            if title_has_content(&el.children) {
                TitleStatus::Present
            } else {
                TitleStatus::Empty
            }
        }
        NodeKind::JinjaBlock(block) => jinja_block_title_status(block),
        _ => TitleStatus::Absent,
    }
}

fn jinja_block_title_status(block: &JinjaBlock<'_, Node<'_>>) -> TitleStatus {
    block
        .body
        .iter()
        .fold(TitleStatus::Absent, |acc, item| match item {
            JinjaTagOrChildren::Children(children) => acc.merge(classify_title(children)),
            JinjaTagOrChildren::Tag(_) => acc,
        })
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

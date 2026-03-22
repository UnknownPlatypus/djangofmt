//! Document structure rules: DOCTYPE and title checks.
//!
//! ## Rules
//!
//! - **H007 / missing-doctype**: Detects HTML documents missing a DOCTYPE declaration.
//! - **H016 / missing-title**: Detects HTML documents with `<head>` but no `<title>`.
//!
//! ## Skipped cases
//!
//! - Template partials (files containing `{% extends %}` or root-level `{% block %}`) are skipped.

use markup_fmt::ast::{Element, Node, NodeKind, Root};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::Violation;

// --- H007: Missing DOCTYPE ---

/// Violation for HTML documents missing a `<!DOCTYPE html>` declaration.
#[derive(Debug, PartialEq, Eq)]
pub struct MissingDoctype;

impl Violation for MissingDoctype {
    const RULE: Rule = Rule::MissingDoctype;
    const CATEGORY: RuleCategory = RuleCategory::Style;

    fn message(&self) -> String {
        "Missing DOCTYPE declaration.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Add `<!DOCTYPE html>` before the `<html>` tag.".to_string())
    }
}

// --- H016: Missing title ---

/// Violation for `<head>` elements missing a `<title>` child.
#[derive(Debug, PartialEq, Eq)]
pub struct MissingTitle;

impl Violation for MissingTitle {
    const RULE: Rule = Rule::MissingTitle;
    const CATEGORY: RuleCategory = RuleCategory::Style;

    fn message(&self) -> String {
        "Missing <title> tag inside <head>.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Add a `<title>` element inside `<head>`.".to_string())
    }
}

/// Check for DOCTYPE when `<html>` is present (H007).
pub fn check_doctype(root: &Root<'_>, checker: &mut Checker<'_>) {
    if is_template_partial(root) {
        return;
    }

    let has_html = root.children.iter().any(|n| {
        matches!(
            &n.kind,
            NodeKind::Element(el) if el.tag_name.eq_ignore_ascii_case("html")
        )
    });
    let has_doctype = root
        .children
        .iter()
        .any(|n| matches!(&n.kind, NodeKind::Doctype(_)));

    if has_html && !has_doctype {
        checker.report(&MissingDoctype, (0, 0).into());
    }
}

/// Check for `<title>` inside any `<head>` element (H016).
pub fn check_title(root: &Root<'_>, checker: &mut Checker<'_>) {
    if is_template_partial(root) {
        return;
    }

    // Find <head> elements (may be nested inside <html>)
    for node in &root.children {
        if let NodeKind::Element(el) = &node.kind {
            if el.tag_name.eq_ignore_ascii_case("html") {
                // Look for <head> inside <html>
                for child in &el.children {
                    check_head_for_title(child, checker);
                }
            } else if el.tag_name.eq_ignore_ascii_case("head") {
                check_head_element(el, checker);
            }
        }
    }
}

fn check_head_for_title(node: &Node<'_>, checker: &mut Checker<'_>) {
    if let NodeKind::Element(el) = &node.kind
        && el.tag_name.eq_ignore_ascii_case("head")
    {
        check_head_element(el, checker);
    }
}

fn check_head_element(head: &Element<'_>, checker: &mut Checker<'_>) {
    let has_title = head.children.iter().any(|child| {
        matches!(
            &child.kind,
            NodeKind::Element(el) if el.tag_name.eq_ignore_ascii_case("title")
        )
    });

    if !has_title {
        let offset = checker.source_offset(head.tag_name);
        checker.report(&MissingTitle, (offset, head.tag_name.len()).into());
    }
}

/// Returns true if the template is a partial (extends or has root-level blocks).
fn is_template_partial(root: &Root<'_>) -> bool {
    root.children.iter().any(|n| match &n.kind {
        NodeKind::JinjaTag(tag) => tag.content.trim().starts_with("extends"),
        NodeKind::JinjaBlock(_) => true,
        _ => false,
    })
}

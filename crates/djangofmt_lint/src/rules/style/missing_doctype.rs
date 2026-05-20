use markup_fmt::ast::{NodeKind, Root};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for HTML documents that contain an `<html>` tag but no `<!DOCTYPE html>` declaration.
///
/// ## Why is this bad?
/// HTML5 requires a DOCTYPE declaration at the top of every document. Without it, browsers fall
/// back to "quirks mode", which emulates legacy rendering bugs and applies different CSS box-model
/// rules. The result is inconsistent layout across browsers and behaviour that is hard to debug.
///
/// Template partials (files with a root-level `{% extends %}` tag or `{% block %}` block) are
/// assumed to inherit the DOCTYPE from their parent template and are not flagged.
///
/// ## Example
/// ```html
/// <html lang="en">
///   <head><title>Page</title></head>
///   <body>Content</body>
/// </html>
/// ```
///
/// Use instead:
/// ```html
/// <!DOCTYPE html>
/// <html lang="en">
///   <head><title>Page</title></head>
///   <body>Content</body>
/// </html>
/// ```
///
/// ## References
/// - [HTML spec: The DOCTYPE](https://html.spec.whatwg.org/multipage/syntax.html#the-doctype)
/// - [MDN: Doctype](https://developer.mozilla.org/en-US/docs/Glossary/Doctype)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct MissingDoctype;

impl Violation for MissingDoctype {
    const RULE: Rule = Rule::MissingDoctype;
    const CATEGORY: RuleCategory = RuleCategory::Style;

    #[derive_message_formats]
    fn message(&self) -> String {
        "Missing `<!DOCTYPE html>` declaration.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Add `<!DOCTYPE html>` before the `<html>` tag.".to_string())
    }
}

pub fn check(root: &Root<'_>, checker: &Checker<'_>) {
    let mut html_element = None;
    let mut has_doctype = false;

    for node in &root.children {
        match &node.kind {
            NodeKind::JinjaBlock(_) => return,
            NodeKind::JinjaTag(tag) if is_extends_tag(tag.content) => return,
            NodeKind::Doctype(_) => has_doctype = true,
            NodeKind::Element(el)
                if html_element.is_none() && el.tag_name.eq_ignore_ascii_case("html") =>
            {
                html_element = Some(el);
            }
            _ => {}
        }
    }

    if has_doctype {
        return;
    }

    let Some(html) = html_element else {
        return;
    };

    let offset = checker.source_offset(html.tag_name);
    checker.report_diagnostic(&MissingDoctype, (offset, html.tag_name.len()).into());
}

/// Returns `true` if `content` is the body of a Jinja `{% extends %}` tag.
///
/// Handles Jinja's whitespace-stripping markers (`{%-` / `-%}`), which the parser preserves as
/// leading/trailing `-` inside `content`. The check after `extends` requires a word boundary so
/// e.g. `{% extendsfoo %}` is not matched.
fn is_extends_tag(content: &str) -> bool {
    let trimmed = content
        .trim_start()
        .strip_prefix('-')
        .map_or_else(|| content.trim_start(), str::trim_start);
    let Some(after_extends) = trimmed.strip_prefix("extends") else {
        return false;
    };
    after_extends
        .chars()
        .next()
        .is_none_or(|c| c.is_ascii_whitespace())
}

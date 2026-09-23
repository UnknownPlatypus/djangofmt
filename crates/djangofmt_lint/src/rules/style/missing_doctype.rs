use std::borrow::Cow;

use markup_fmt::ast::{NodeKind, Root};
use markup_fmt::parser::parse_jinja_tag_name;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
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
/// The declaration must come before the `<html>` tag: one placed after it still leaves the
/// browser in quirks mode.
///
/// Files that extend another template are assumed to inherit the DOCTYPE from their parent and
/// are not flagged. Neither is a document whose `<html>` tag is preceded by a root-level `{% %}`
/// block, since the block may be the one emitting the DOCTYPE.
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
/// ## Fix safety
/// The fix is marked unsafe: switching a document out of quirks mode is the point of the rule, but
/// it changes how the page renders, and a layout tuned against quirks-mode box sizing may shift.
///
/// ## References
/// - [HTML spec: The DOCTYPE](https://html.spec.whatwg.org/multipage/syntax.html#the-doctype)
/// - [MDN: Doctype](https://developer.mozilla.org/en-US/docs/Glossary/Doctype)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.9")]
pub struct MissingDoctype;

impl Violation for MissingDoctype {
    const RULE: Rule = Rule::MissingDoctype;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        "Missing `<!DOCTYPE html>` declaration".into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some("Add `<!DOCTYPE html>` before the `<html>` tag".into())
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Add `<!DOCTYPE html>` declaration")
    }
}

pub fn check(checker: &Checker<'_>, root: &Root<'_>) {
    // The first of these nodes settles the file: a DOCTYPE declared after the `<html>` tag comes
    // too late, and a `{% %}` block before it may be the one emitting the DOCTYPE.
    for node in &root.children {
        match &node.kind {
            NodeKind::Doctype(_) | NodeKind::JinjaBlock(_) => return,
            NodeKind::JinjaTag(tag) if parse_jinja_tag_name(tag) == "extends" => return,
            NodeKind::Element(el) if el.tag_name.eq_ignore_ascii_case("html") => {
                let mut guard =
                    checker.report_diagnostic(&MissingDoctype, checker.source_span(el.tag_name));
                if !has_late_doctype(root) {
                    guard.set_fix(Fix::unsafe_edit(Edit::insertion(
                        "<!DOCTYPE html>\n",
                        checker.source_offset(node.raw),
                    )));
                }
                return;
            }
            _ => {}
        }
    }
}

fn has_late_doctype(root: &Root<'_>) -> bool {
    root.children
        .iter()
        .any(|node| matches!(node.kind, NodeKind::Doctype(_)))
}

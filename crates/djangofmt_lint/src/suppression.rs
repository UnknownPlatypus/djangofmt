//! Diagnostic suppression via `{# djangofmt: ignore[...] #}` comments.
//!
//! Some details:
//! - A directive guards the node that follows it.
//! - `file-ignore[...]` as the file's first comment covers the whole file.
//! - Only `{# #}` comments carry directives: HTML comments reach the client.
use std::ops::Range;
use std::str::FromStr;

use djangofmt_syntax::{IgnoreDirective, TEMPLATE_COMMENT, strip_bom};
use markup_fmt::ast::{JinjaTagOrChildren, Node, NodeKind, Root};
use smallvec::{SmallVec, smallvec};

use crate::registry::Rule;
use crate::rule_set::RuleSet;
use crate::rules;
use crate::rules::helpers::enclosing_comment;
use crate::{LintContext, LintDiagnostic};

/// How far a directive in force reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoreScope {
    /// An `ignore[...]`, covering the node that follows it.
    Node,
    /// A leading `file-ignore[...]`, covering the whole file.
    File,
}

/// How far `directive` reaches, `None` when it is misplaced or malformed and so silences nothing.
const fn scope(directive: &IgnoreDirective<'_>, is_leading: bool) -> Option<IgnoreScope> {
    match directive {
        IgnoreDirective::Ignore(_) => Some(IgnoreScope::Node),
        IgnoreDirective::FileIgnore(_) if is_leading => Some(IgnoreScope::File),
        IgnoreDirective::FileIgnore(_) | IgnoreDirective::Malformed(_) => None,
    }
}

/// Whether free text follows the code list of a directive `body`; Jinja's trailing
/// whitespace-control marker (`-#}`) is part of the delimiter, not a reason.
fn has_reason(body: &str) -> bool {
    body.split_once(']')
        .is_some_and(|(_, tail)| !tail.strip_suffix('-').unwrap_or(tail).trim().is_empty())
}

/// A `{# djangofmt: ... #}` comment as the linter reads it.
#[derive(Debug)]
pub struct IgnoreComment<'s> {
    /// The whole comment, delimiters included: what a diagnostic points at and a fix deletes.
    pub raw: &'s str,
    body: &'s str,
    /// What the comment asks for.
    pub directive: IgnoreDirective<'s>,
    /// How far the directive reaches, `None` when it silences nothing.
    pub scope: Option<IgnoreScope>,
    /// The rules of the diagnostics this comment silences.
    pub matched: RuleSet,
    /// Byte ranges the directive guards: the whole file for a leading `file-ignore[...]`,
    /// the next node for an `ignore[...]`, nothing when it has no target.
    guarded_ranges: SmallVec<[Range<usize>; 2]>,
}

impl IgnoreComment<'_> {
    /// Whether free text follows the code list, e.g. `ignore[x]: why`.
    #[must_use]
    pub fn has_reason(&self) -> bool {
        has_reason(self.body)
    }

    /// Whether the comment silences `diagnostic`.
    fn suppresses(&self, diagnostic: &LintDiagnostic) -> bool {
        let offset = diagnostic.span.offset() as usize;
        self.guarded_ranges
            .iter()
            .any(|range| range.contains(&offset))
            && self.directive.codes().contains(&diagnostic.code)
    }
}

/// Lint the ignore comments, then drop the diagnostics they silence.
pub fn check_ignores(context: &LintContext<'_>, root: &Root<'_>) {
    let mut comments = collect_ignore_comments(root, context);
    if context.is_rule_enabled(Rule::InvalidIgnoreComment) {
        rules::suspicious::invalid_ignore_comment::check(context, &comments);
    }
    if context.is_rule_enabled(Rule::InvalidIgnoreCode) {
        rules::suspicious::invalid_ignore_code::check(context, &comments);
    }
    record_matches(context, &mut comments);
    if context.is_rule_enabled(Rule::UnusedIgnoreCode) {
        rules::suspicious::unused_ignore_code::check(context, &comments);
    }
    drop_ignored_diagnostics(context, &comments);
}

/// Every `{# djangofmt: ... #}` comment of the file, in source order.
fn collect_ignore_comments<'s>(
    root: &Root<'s>,
    context: &LintContext<'s>,
) -> Vec<IgnoreComment<'s>> {
    let source = context.source();
    root.jinja_comments
        .iter()
        .filter_map(|comment_body| {
            let directive = IgnoreDirective::parse(comment_body)?;
            let offset = context.source_offset(comment_body);

            let raw = enclosing_comment(context, TEMPLATE_COMMENT, comment_body);
            let is_leading = strip_bom(&source[..context.source_offset(raw)])
                .trim_start()
                .is_empty();
            let scope = scope(&directive, is_leading);
            let guarded_ranges = match scope {
                Some(IgnoreScope::Node) => guarded_ranges(root, offset, context),
                Some(IgnoreScope::File) => smallvec![0..source.len()],
                None => SmallVec::new(),
            };
            Some(IgnoreComment {
                raw,
                body: comment_body,
                directive,
                scope,
                matched: RuleSet::empty(),
                guarded_ranges,
            })
        })
        .collect()
}

/// Record on each comment the rules of the diagnostics it silences. Dropping them waits for
/// [`drop_ignored_diagnostics`], once `unused-ignore-code` has reported from these records.
/// A diagnostic counts for the first comment covering it, so a repeated code is unused.
fn record_matches(context: &LintContext<'_>, comments: &mut [IgnoreComment<'_>]) {
    for diagnostic in context.diagnostics().iter() {
        if let Ok(rule) = Rule::from_str(diagnostic.code)
            && let Some(comment) = comments.iter_mut().find(|c| c.suppresses(diagnostic))
        {
            comment.matched.insert(rule);
        }
    }
}

/// Drop from `context` the diagnostics the comments silence, those of the rules on the
/// comments themselves included.
fn drop_ignored_diagnostics(context: &LintContext<'_>, comments: &[IgnoreComment<'_>]) {
    if comments.is_empty() {
        return;
    }
    context.retain_diagnostics(|diagnostic| !comments.iter().any(|c| c.suppresses(diagnostic)));
}

/// The siblings following the `{# #}` comment node whose body sits at `offset`.
fn siblings_after<'n, 's>(
    mut nodes: &'n [Node<'s>],
    offset: usize,
    context: &LintContext<'_>,
) -> Option<&'n [Node<'s>]> {
    let start = |node: &Node<'_>| context.source_offset(node.raw);
    let end = |node: &Node<'_>| context.source_end(node.raw);
    loop {
        // Siblings are ordered and disjoint: the container is the first one ending past `offset`.
        let index = nodes.partition_point(|node| end(node) <= offset);
        let node = nodes.get(index).filter(|node| start(node) <= offset)?;
        if matches!(node.kind, NodeKind::JinjaComment(_)) {
            return Some(&nodes[index + 1..]);
        }
        nodes = child_slices(node).find(|children| {
            children
                .first()
                .zip(children.last())
                .is_some_and(|(first, last)| start(first) <= offset && offset < end(last))
        })?;
    }
}

/// What the comment at `offset` guards: the first node after it and the HTML comments in between,
/// minus that node's children other than text and HTML comments, which can hold no ignore comment.
fn guarded_ranges(
    root: &Root<'_>,
    offset: usize,
    context: &LintContext<'_>,
) -> SmallVec<[Range<usize>; 2]> {
    let mut ranges = SmallVec::new();
    // At attribute position the comment is no node, so it has no siblings and guards nothing.
    let Some(siblings) = siblings_after(&root.children, offset, context) else {
        return ranges;
    };
    let mut target = None;
    for node in siblings {
        match &node.kind {
            // Django reads the tags in an HTML comment too, and no ignore comment can bind to one.
            NodeKind::Comment(_) => {
                ranges.push(context.source_offset(node.raw)..context.source_end(node.raw));
            }
            NodeKind::JinjaComment(_) => {}
            _ if is_whitespace_text(node) => {}
            _ => {
                target = Some(node);
                break;
            }
        }
    }
    let Some(target) = target else {
        return ranges;
    };
    let start = context.source_offset(target.raw);
    let end = context.source_end(target.raw);
    let mut cursor = start;
    for child in child_slices(target).flatten() {
        if matches!(child.kind, NodeKind::Text(_) | NodeKind::Comment(_)) {
            continue;
        }
        let child_start = context.source_offset(child.raw);
        if child_start > cursor {
            ranges.push(cursor..child_start);
        }
        cursor = context.source_end(child.raw);
    }
    if cursor < end {
        ranges.push(cursor..end);
    }
    ranges
}

/// The child slices of a node: an element's children, or each branch of a block.
fn child_slices<'n, 's>(node: &'n Node<'s>) -> impl Iterator<Item = &'n [Node<'s>]> {
    let (children, body) = match &node.kind {
        NodeKind::Element(element) => (Some(element.children.as_slice()), None),
        NodeKind::JinjaBlock(block) => (None, Some(block.body.as_slice())),
        _ => (None, None),
    };
    children
        .into_iter()
        .chain(body.into_iter().flatten().filter_map(|item| match item {
            JinjaTagOrChildren::Children(children) => Some(children.as_slice()),
            JinjaTagOrChildren::Tag(_) => None,
        }))
}

/// Whitespace between an ignore comment and its target does not displace the target.
fn is_whitespace_text(node: &Node<'_>) -> bool {
    matches!(node.kind, NodeKind::Text(_)) && node.raw.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Settings, lint_source};
    use markup_fmt::Language;
    use rstest::rstest;

    #[rstest]
    #[case::reason("djangofmt: ignore[a]: free-text reason", true)]
    #[case::none(" djangofmt: ignore[a] ", false)]
    #[case::whitespace_control("- djangofmt: ignore[a] -", false)]
    // Only a hyphen glued to `#}` is whitespace control; spaced out it is free text.
    #[case::spaced_hyphen(" djangofmt: ignore[a] - ", true)]
    fn a_reason_follows_the_code_list(#[case] body: &str, #[case] expected: bool) {
        assert_eq!(has_reason(body), expected);
    }

    /// A leading `file-ignore[...]` covers the whole file, nested nodes included.
    #[test]
    fn file_ignore_covers_the_whole_file() {
        assert_eq!(
            codes(
                "{# djangofmt: file-ignore[invalid-attr-value] #}\n\
                 <form method=\"yes\"></form>\n\
                 <div><form method=\"put\"></form></div>"
            ),
            Vec::<&str>::new()
        );
        // Below the file's top it covers nothing, and is reported as misplaced.
        assert_eq!(
            codes(
                "<p>hi</p>\n{# djangofmt: file-ignore[invalid-attr-value] #}\n<form method=\"yes\"></form>"
            ),
            ["invalid-ignore-comment", "invalid-attr-value"]
        );
    }

    /// Django reads the tags inside a `<script>` body and an HTML comment,
    /// where no ignore comment can bind, so the enclosing node's ignore covers them.
    #[test]
    fn node_ignore_covers_the_text_and_comments_inside_it() {
        assert_eq!(
            codes(
                "{# djangofmt: ignore[duplicate-block-name] #}\n\
                 <script>{% block a %}{% endblock %}{% block a %}{% endblock %}</script>"
            ),
            Vec::<&str>::new()
        );
        assert_eq!(
            codes(
                "{# djangofmt: ignore[duplicate-block-name] #}\n\
                 <div><!-- {% block a %}{% block a %} --></div>"
            ),
            Vec::<&str>::new()
        );
    }

    /// An ignore comment reaches past an HTML comment to its target, and covers both.
    #[test]
    fn node_ignore_covers_the_html_comments_before_its_target() {
        assert_eq!(
            codes(
                "{# djangofmt: ignore[duplicate-block-name, invalid-attr-value] #}\n\
                 <!-- {% block a %}{% block a %} -->\n<form method=\"yes\"></form>"
            ),
            Vec::<&str>::new()
        );
    }

    /// The rules on directives are suppressed like any other, by a `file-ignore[...]`.
    #[test]
    fn file_ignore_silences_the_directive_rules() {
        assert_eq!(
            codes(
                "{# djangofmt: file-ignore[invalid-ignore-code] #}\n{# djangofmt: ignore[not-a-rule] #}\n<p>hi</p>"
            ),
            Vec::<&str>::new()
        );
    }

    /// Of two comments silencing the same diagnostic, the second is the unused one.
    #[test]
    fn a_diagnostic_counts_for_its_first_comment_only() {
        assert_eq!(
            messages(
                "{# djangofmt: file-ignore[invalid-attr-value] #}\n{# djangofmt: ignore[invalid-attr-value] #}\n<form method=\"yes\"></form>"
            ),
            ["Unused rule code in suppression: `invalid-attr-value`"]
        );
    }

    /// `<form method="yes">` trips `invalid-attr-value`, `id=""` trips `empty-attr-value`.
    fn codes(source: &str) -> Vec<&'static str> {
        diagnostics(source)
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    /// Diagnostics by message, to tell apart two that share a code.
    fn messages(source: &str) -> Vec<String> {
        diagnostics(source)
            .into_iter()
            .map(|diagnostic| diagnostic.message.into_owned())
            .collect()
    }

    fn diagnostics(source: &str) -> Vec<LintDiagnostic> {
        lint_source(source, Language::Django, &[], &Settings::all(), None).expect("parse")
    }

    /// Suppression does not hinge on the rules about ignore comments being on.
    #[test]
    fn ignore_applies_with_the_directive_rules_off() {
        let settings = Settings {
            rules: RuleSet::from_rule(Rule::InvalidAttrValue),
            ..Settings::all()
        };
        let source = "{# djangofmt: ignore[invalid-attr-value] #}\n<form method=\"yes\"></form>";
        let diagnostics = lint_source(source, Language::Django, &[], &settings, None);
        assert!(diagnostics.expect("parse").is_empty());
    }

    #[test]
    fn ignore_binds_to_the_following_node() {
        assert_eq!(
            codes("<form method=\"yes\"></form>"),
            ["invalid-attr-value"]
        );
        assert_eq!(
            codes("{# djangofmt: ignore[invalid-attr-value] #}\n<form method=\"yes\"></form>"),
            Vec::<&str>::new()
        );
        // A non-matching code, an ignore comment placed after the node, and a later
        // sibling all keep their diagnostic; the first two comments are unused on top.
        assert_eq!(
            codes("{# djangofmt: ignore[empty-attr-value] #}\n<form method=\"yes\"></form>"),
            ["unused-ignore-code", "invalid-attr-value"]
        );
        assert_eq!(
            codes("<form method=\"yes\"></form>\n{# djangofmt: ignore[invalid-attr-value] #}"),
            ["invalid-attr-value", "unused-ignore-code"]
        );
        assert_eq!(
            messages(
                "{# djangofmt: ignore[invalid-attr-value] #}\n<form method=\"yes\"></form>\n<form method=\"put\"></form>"
            ),
            ["Invalid value `put` for attribute `method`"]
        );
    }

    #[test]
    fn ignore_reaches_past_intervening_comments() {
        for filler in ["{# TODO: fix this legacy form #}", "<!-- legacy form -->"] {
            let source = format!(
                "{{# djangofmt: ignore[invalid-attr-value] #}}\n{filler}\n<form method=\"yes\"></form>"
            );
            assert!(!codes(&source).contains(&"invalid-attr-value"), "{filler}");
        }
        // Text is a node of its own, so it takes the guard instead.
        assert_eq!(
            codes(
                "{# djangofmt: ignore[invalid-attr-value] #}\nLegacy: <form method=\"yes\"></form>"
            ),
            ["unused-ignore-code", "invalid-attr-value"]
        );
        // Stacked ignore comments all reach the same target.
        assert_eq!(
            codes(
                "{# djangofmt: ignore[invalid-attr-value] #}\n{# djangofmt: ignore[empty-attr-value] #}\n<form method=\"yes\" id=\"\"></form>"
            ),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn ignore_applies_inside_nested_nodes() {
        const GUARDED: &str =
            "{# djangofmt: ignore[invalid-attr-value] #}\n<form method=\"yes\"></form>";
        assert_eq!(
            codes(&format!("<div>\n{GUARDED}\n</div>")),
            Vec::<&str>::new()
        );
        // In a later branch too, past the siblings of the first one.
        assert_eq!(
            codes(&format!(
                "{{% if x %}}<p>x</p>{{% else %}}\n{GUARDED}\n{{% endif %}}"
            )),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn ignore_guards_the_node_but_not_its_children() {
        // The comment guards the `<div>` tags, not the `<span>` nested inside them.
        assert_eq!(
            messages(
                "{# djangofmt: ignore[empty-attr-value] #}\n<div id=\"\"><span class=\"\">x</span></div>"
            ),
            ["Empty `class` attribute"]
        );
        // Likewise a block guards its own tags but not its body.
        assert_eq!(
            codes(
                "{# djangofmt: ignore[untrimmed-blocktranslate] #}\n{% blocktranslate %}x{% endblocktranslate %}"
            ),
            Vec::<&str>::new()
        );
        assert_eq!(
            codes(
                "{# djangofmt: ignore[invalid-attr-value] #}\n{% if x %}<form method=\"yes\"></form>{% endif %}"
            ),
            ["unused-ignore-code", "invalid-attr-value"]
        );
    }

    #[test]
    fn whitespace_control_markers_do_not_break_suppression() {
        assert_eq!(
            codes("{#- djangofmt: ignore[invalid-attr-value] -#}\n<form method=\"yes\"></form>"),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn html_comments_never_suppress() {
        assert!(
            codes("<!-- djangofmt: ignore[invalid-attr-value] -->\n<form method=\"yes\"></form>")
                .contains(&"invalid-attr-value")
        );
    }

    /// An attribute-position comment is indexed by the parser but is no node, so it has no target.
    #[test]
    fn attribute_position_comments_never_suppress() {
        assert_eq!(
            codes("<form {# djangofmt: ignore[invalid-attr-value] #} method=\"yes\"></form>"),
            ["unused-ignore-code", "invalid-attr-value"]
        );
    }
}

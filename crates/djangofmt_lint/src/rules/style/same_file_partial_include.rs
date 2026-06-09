use markup_fmt::ast::JinjaTag;
use markup_fmt::parser::parse_jinja_tag_name;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `{% include %}` tags that render a partial defined in the same template file: the
/// include's template path (before `#`) is a suffix of the linted file's path and a matching
/// `{% partialdef %}` exists in the file. Dynamic template names and includes passing `with` /
/// `only` context are left alone, and the rule is skipped when the file path is unknown (e.g.
/// the playground).
///
/// ## Why is this bad?
/// `{% include "file.html#name" %}` reloads the template from disk to extract the `name` partial.
/// When that partial lives in the same file, `{% partial name %}` renders it directly without the
/// extra load and makes the same-file relationship explicit.
///
/// ## Example
/// ```html
/// {% partialdef item-list %}...{% endpartialdef %}
/// {% include "my_app/items_list.html#item-list" %}
/// ```
///
/// Use instead:
/// ```html
/// {% partialdef item-list %}...{% endpartialdef %}
/// {% partial item-list %}
/// ```
///
/// ## Fix safety
/// This rule's fix is marked as safe: it only fires when the include's template path matches the
/// file being linted and the partial is defined in it, so `{% partial %}` renders the same partial
/// `{% include %}` would load. Whitespace-control markers (`{%-` / `-%}`) are preserved.
///
/// ## References
/// - [django-template-partials](https://github.com/carltongibson/django-template-partials)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(preview_since = "0.2.11")]
pub struct SameFilePartialInclude<'a> {
    pub name: &'a str,
}

impl Violation for SameFilePartialInclude<'_> {
    const RULE: Rule = Rule::SameFilePartialInclude;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> String {
        format!(
            "Same-file partial `{}` rendered via `{{% include %}}`.",
            self.name
        )
    }

    fn help(&self) -> Option<String> {
        Some(format!(
            "Render it with `{{% partial {} %}}` to avoid reloading the template from disk.",
            self.name
        ))
    }

    fn fix_title(&self) -> Option<String> {
        Some("Replace `{% include %}` with `{% partial %}`".to_string())
    }
}

pub fn check(tag: &JinjaTag<'_>, checker: &Checker<'_>) {
    // Same-file detection needs the linted file's path (absent in e.g. the WASM playground).
    let Some(current_path) = checker.context().path() else {
        return;
    };

    let tag_name = parse_jinja_tag_name(tag);
    if tag_name != "include" {
        return;
    }

    let Some((template_path, fragment)) = parse_partial_include(tag, tag_name) else {
        return;
    };

    if !current_path.ends_with(template_path) {
        return;
    }

    // Without a local partialdef, the suffix-matched include necessarily resolves to another file.
    if !defines_partial(checker.context().source(), fragment) {
        return;
    }

    let span = (tag.start - "{%".len(), tag.content.len() + "{%%}".len()).into();
    let mut guard = checker.report_diagnostic(&SameFilePartialInclude { name: fragment }, span);

    // Carry the tag's whitespace-control markers over to the replacement.
    let lead = whitespace_control_marker(tag.content.chars().next());
    let trail = whitespace_control_marker(tag.content.chars().next_back());
    guard.set_fix(Fix::safe_edit(Edit::replacement(
        format!("{{%{lead} partial {fragment} {trail}%}}"),
        span,
    )));
}

/// The whitespace-control marker (`-` / `+`) at the given edge of a tag's `content`, or `""`.
const fn whitespace_control_marker(edge: Option<char>) -> &'static str {
    match edge {
        Some('-') => "-",
        Some('+') => "+",
        _ => "",
    }
}

/// Split an include tag into `(template_path, fragment)`, or [`None`] if it is not static.
fn parse_partial_include<'s>(tag: &JinjaTag<'s>, tag_name: &str) -> Option<(&'s str, &'s str)> {
    // The arguments after the tag name, with whitespace-control markers stripped from both edges.
    let args = tag
        .content
        .trim_matches(['-', '+'])
        .trim()
        .strip_prefix(tag_name)?
        .trim();

    // The template name must be a string literal; a variable name is dynamic and left alone.
    let quote = args.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let (template_ref, rest) = args[1..].split_once(quote)?;

    // Trailing tokens (`with`, `only`, a filter, ...) and interpolation have no `{% partial %}`
    // equivalent.
    if !rest.trim().is_empty() || contains_interpolation(template_ref) {
        return None;
    }

    let (template_path, fragment) = template_ref.split_once('#')?;
    if template_path.is_empty() || fragment.is_empty() || fragment.contains(char::is_whitespace) {
        return None;
    }

    Some((template_path, fragment))
}

/// Whether the source contains a `{% partialdef <name> %}` opening tag.
fn defines_partial(source: &str, name: &str) -> bool {
    // Each `{%`-delimited chunk that opens with `partialdef <name>` as whole tokens. Splitting on
    // `{%` rejects `endpartialdef` for free (its chunk starts with `end`).
    source.split("{%").skip(1).any(|tag| {
        tag.trim_start_matches(['-', '+'])
            .trim_start()
            .strip_prefix("partialdef")
            .filter(|rest| rest.starts_with(char::is_whitespace))
            .map(str::trim_start)
            .and_then(|rest| rest.strip_prefix(name))
            .is_some_and(|rest| rest.starts_with(|c: char| c.is_whitespace() || c == '%'))
    })
}

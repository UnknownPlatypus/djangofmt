use std::borrow::Cow;
use std::path::Path;

use markup_fmt::ast::JinjaTag;

use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};
use djangofmt_syntax::dtl::bits;
use djangofmt_syntax::span;

use crate::Checker;

/// ## What it does
/// Checks for `{% include %}` tags that render a partial defined in the same template file.
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
/// ## References
/// - [django-template-partials](https://github.com/carltongibson/django-template-partials)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "1.0.0")]
pub struct SameFilePartialInclude<'a> {
    pub name: &'a str,
}

impl Violation for SameFilePartialInclude<'_> {
    const RULE: Rule = Rule::SameFilePartialInclude;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        format!(
            "Same-file partial `{}` rendered via `{{% include %}}`",
            self.name
        )
        .into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some(format!("Use `{{% partial {} %}}` instead", self.name).into())
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Replace `{% include %}` with `{% partial %}`")
    }
}

/// The caller guarantees the tag is an `include`.
pub fn check(checker: &Checker<'_>, tag: &JinjaTag<'_>, current_path: &Path) {
    // Every include gets here and only a `#fragment` names a partial, so the rest runs out of line.
    if tag.content.contains('#') {
        check_include(checker, tag, current_path);
    }
}

#[cold]
fn check_include(checker: &Checker<'_>, tag: &JinjaTag<'_>, current_path: &Path) {
    let Some((template_path, fragment)) = parse_partial_include(tag) else {
        return;
    };

    if !current_path.ends_with(template_path) {
        return;
    }

    // Without a local partialdef, the suffix-matched include necessarily resolves to another file.
    if !defines_partial(checker.context().source(), fragment) {
        return;
    }

    let span = span(tag.start - "{%".len(), tag.content.len() + "{%%}".len());
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
fn parse_partial_include<'s>(tag: &JinjaTag<'s>) -> Option<(&'s str, &'s str)> {
    // Whitespace-control markers are stripped first, or they would lex as bits of their own.
    // Trailing bits (`with`, `only`, ...) have no `{% partial %}` equivalent.
    let [_, template] = bits(tag.content.trim_matches(['-', '+']))[..] else {
        return None;
    };

    // The template name must be a string literal; a variable name is dynamic and left alone.
    let quote = template.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let (template_ref, rest) = template[1..].split_once(quote)?;

    // A filter after the string, or interpolation inside it, has no equivalent either.
    if !rest.is_empty() || contains_interpolation(template_ref) {
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

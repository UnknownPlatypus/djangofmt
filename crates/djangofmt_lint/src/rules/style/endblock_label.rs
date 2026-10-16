use std::borrow::Cow;

use djangofmt_syntax::dtl::bits;
use djangofmt_syntax::span;
use markup_fmt::ast::{JinjaBlock, JinjaTagOrChildren};

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::tag_tokens;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `{% endblock %}` tags that repeat the block's name on a single-line block, or omit
/// it on a block spanning several lines.
///
/// ## Why is this bad?
/// On one line, the opening tag is in view and the label is noise. Over several lines, the opening
/// tag may be out of view, and the label tells which block ends.
///
/// Also applies to `{% endpartialdef %}` and to `custom-blocks` taking one argument.
///
/// ## Example
/// ```html
/// {% block content %}
///     <h1>{% block title %}Home{% endblock title %}</h1>
/// {% endblock %}
/// ```
///
/// Use instead:
/// ```html
/// {% block content %}
///     <h1>{% block title %}Home{% endblock %}</h1>
/// {% endblock content %}
/// ```
///
/// ## Options
/// - `custom-blocks`
///
/// ## References
/// - [Django coding style: template style](https://docs.djangoproject.com/en/stable/internals/contributing/writing-code/coding-style/#template-style)
/// - [Django documentation: template inheritance](https://docs.djangoproject.com/en/stable/ref/templates/language/#template-inheritance)
/// - [Django documentation: template partials](https://docs.djangoproject.com/en/stable/ref/templates/language/#template-partials)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct EndblockLabel<'a> {
    pub end_tag: &'a str,
    pub kind: LabelViolation<'a>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LabelViolation<'a> {
    Missing { name: &'a str },
    Redundant,
}

impl Violation for EndblockLabel<'_> {
    const RULE: Rule = Rule::EndblockLabel;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        match self.kind {
            LabelViolation::Missing { .. } => "Missing end tag label on a multi-line block".into(),
            LabelViolation::Redundant => "Redundant end tag label on a single-line block".into(),
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        let (sep, name) = match self.kind {
            LabelViolation::Missing { name } => (" ", name),
            LabelViolation::Redundant => ("", ""),
        };
        Some(
            ["Write it as `{% ", self.end_tag, sep, name, " %}`"]
                .concat()
                .into(),
        )
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some(match self.kind {
            LabelViolation::Missing { .. } => "Add label",
            LabelViolation::Redundant => "Remove label",
        })
    }
}

pub fn check<T>(checker: &Checker<'_>, block: &JinjaBlock<'_, T>) {
    // A Django style guide convention, Jinja has none.
    if !checker.is_django() {
        return;
    }
    let (Some(JinjaTagOrChildren::Tag(opener)), Some(JinjaTagOrChildren::Tag(closer))) =
        (block.body.first(), block.body.last())
    else {
        return;
    };
    let mut args = tag_tokens(opener.content);
    let Some(tag) = args.next() else {
        return;
    };
    if !matches!(tag, "block" | "partialdef")
        && !checker.custom_blocks().iter().any(|custom| custom == tag)
    {
        return;
    }

    let end = closer.content.trim_ascii();
    let Some(rest) = end
        .strip_prefix("end")
        .and_then(|rest| rest.strip_prefix(tag))
    else {
        return;
    };
    let end_tag = &end[..end.len() - rest.len()];
    // Django matches the whole end tag, so the label follows exactly one space.
    let label = match rest.strip_prefix(' ') {
        None if rest.is_empty() => None,
        Some(label) => Some(label),
        None => return,
    };

    let between = &checker.context().source()
        [checker.source_end(opener.content)..checker.source_offset(closer.content)];
    let same_line = !between.contains('\n');
    if same_line != label.is_some() {
        return;
    }

    // Django rejects other arities. `partialdef` keeps a quoted name whole.
    let name = if tag == "partialdef" {
        match bits(opener.content)[..] {
            [_, name] | [_, name, "inline"] => name,
            _ => return,
        }
    } else {
        match (args.next(), args.next()) {
            (Some(name), None) => name,
            _ => return,
        }
    };

    let tag_end = checker.source_end(end_tag);
    let (kind, span, edit) = match label {
        None => (
            LabelViolation::Missing { name },
            checker.source_span(end_tag),
            Edit::insertion([" ", name].concat(), tag_end),
        ),
        Some(label) if label == name => (
            LabelViolation::Redundant,
            checker.source_span(label),
            Edit::deletion(span(tag_end, checker.source_end(label) - tag_end)),
        ),
        // A label naming another block is a Django error, left to the author.
        Some(_) => return,
    };
    checker
        .report_diagnostic(&EndblockLabel { end_tag, kind }, span)
        .set_fix(Fix::safe_edit(edit));
}

use std::borrow::Cow;

use djangofmt_syntax::dtl::bits;
use djangofmt_syntax::span;
use markup_fmt::Language;
use markup_fmt::ast::{JinjaBlock, JinjaTagOrChildren};
use markup_fmt::parser::parse_jinja_tag_name;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `{% endblock %}` tags that repeat the block's name on a single-line block, or omit
/// it on a block spanning several lines.
///
/// ## Why is this bad?
/// On one line, the opening tag is in view and the label is noise. Over several lines, the opening
/// tag may be out of view, and the label tells which block ends.
///
/// Also applies to `{% endpartialdef %}`, and to the `custom-blocks` listed in
/// `lint.endblock-label.custom-blocks`. Django rejects an `{% endblock %}` label naming another
/// block, but accepts any label on a custom block, so for those the rule also fixes a label that
/// does not match.
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
/// - `lint.endblock-label.custom-blocks`
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
    Mismatched { name: &'a str },
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
            LabelViolation::Mismatched { .. } => "End tag label does not match the block".into(),
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        let end_tag = self.end_tag;
        Some(
            match self.kind {
                LabelViolation::Missing { name } | LabelViolation::Mismatched { name } => {
                    format!("Write it as `{{% {end_tag} {name} %}}`")
                }
                LabelViolation::Redundant => format!("Write it as `{{% {end_tag} %}}`"),
            }
            .into(),
        )
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some(match self.kind {
            LabelViolation::Missing { .. } => "Add label",
            LabelViolation::Redundant => "Remove label",
            LabelViolation::Mismatched { .. } => "Replace label",
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
    let tag = parse_jinja_tag_name(opener, Language::Django);
    let django_checks_label = matches!(tag, "block" | "partialdef");
    let custom_blocks = &checker.context().settings().endblock_label.custom_blocks;
    if !django_checks_label && !custom_blocks.iter().any(|custom| custom == tag) {
        return;
    }

    let end = closer.content.trim_ascii();
    // Django matches the whole end tag, so the label follows exactly one space.
    let (end_tag, label) = end
        .split_once(' ')
        .map_or((end, None), |(end_tag, label)| (end_tag, Some(label)));
    if end_tag.strip_prefix("end") != Some(tag) {
        return;
    }

    // Django rejects other arities. `partialdef` keeps a quoted name whole.
    let name = match (tag, &bits(opener.content)[..]) {
        (_, [_, name]) | ("partialdef", [_, name, "inline"]) => *name,
        _ => return,
    };

    let between = &checker.context().source()
        [checker.source_end(opener.content)..checker.source_offset(closer.content)];
    let tag_end = checker.source_end(end_tag);
    let (kind, span, edit) = match (between.contains('\n'), label) {
        (true, None) => (
            LabelViolation::Missing { name },
            checker.source_span(end_tag),
            Edit::insertion([" ", name].concat(), tag_end),
        ),
        // A label naming another block is a Django error, left to the author.
        (_, Some(label)) if django_checks_label && label != name => return,
        (false, Some(label)) => (
            LabelViolation::Redundant,
            checker.source_span(label),
            Edit::deletion(span(tag_end, checker.source_end(label) - tag_end)),
        ),
        (true, Some(label)) if label != name => (
            LabelViolation::Mismatched { name },
            checker.source_span(label),
            Edit::replacement(name.to_owned(), checker.source_span(label)),
        ),
        _ => return,
    };
    checker
        .report_diagnostic(&EndblockLabel { end_tag, kind }, span)
        .set_fix(Fix::safe_edit(edit));
}

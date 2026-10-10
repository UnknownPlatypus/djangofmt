use std::borrow::Cow;

use djangofmt_syntax::span;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::{BlockEnd, BlockKind};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `{% endblock %}` and `{% endpartialdef %}` tags that repeat the name of a block
/// written on a single line.
///
/// ## Why is this bad?
/// When the whole block sits on one line, the opening tag is in view of the closing one, so the
/// label repeats what the reader already sees and only makes the line longer. Django's template
/// style guide asks for the label only when the closing tag sits on another line.
///
/// A label that names another block is left alone: Django rejects the template, and only its author
/// knows which block was meant.
///
/// ## Known problems
/// The label is decided from the layout at fix time: in a pre-commit hook that runs `check --fix`
/// before the formatter, a block the formatter then joins onto one line loses its label on the
/// next run.
///
/// ## Example
/// ```html
/// <title>{% block title %}Home{% endblock title %}</title>
/// ```
///
/// Use instead:
/// ```html
/// <title>{% block title %}Home{% endblock %}</title>
/// ```
///
/// ## References
/// - [Django coding style: template style](https://docs.djangoproject.com/en/stable/internals/contributing/writing-code/coding-style/#template-style)
/// - [Django documentation: template inheritance](https://docs.djangoproject.com/en/stable/ref/templates/language/#template-inheritance)
/// - [Django documentation: template partials](https://docs.djangoproject.com/en/stable/ref/templates/language/#template-partials)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct RedundantEndblockLabel {
    pub kind: BlockKind,
}

impl Violation for RedundantEndblockLabel {
    const RULE: Rule = Rule::RedundantEndblockLabel;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        match self.kind {
            BlockKind::Block => "Redundant label on `{% endblock %}` of a single-line block".into(),
            BlockKind::Partialdef => {
                "Redundant label on `{% endpartialdef %}` of a single-line block".into()
            }
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some(match self.kind {
            BlockKind::Block => "Write it as `{% endblock %}`".into(),
            BlockKind::Partialdef => "Write it as `{% endpartialdef %}`".into(),
        })
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Remove label")
    }
}

/// The caller guarantees a Django block whose closer carries `label` on the opener's line.
pub fn check(checker: &Checker<'_>, end: &BlockEnd<'_>, label: &str) {
    let mut guard = checker.report_diagnostic(
        &RedundantEndblockLabel { kind: end.kind },
        checker.source_span(label),
    );
    // From the end of the tag name, so the whitespace before the label goes with it.
    let start = checker.source_end(end.end_tag);
    guard.set_fix(Fix::safe_edit(Edit::deletion(span(
        start,
        checker.source_end(label) - start,
    ))));
}

use std::borrow::Cow;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::{BlockEnd, BlockKind};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `{% endblock %}` and `{% endpartialdef %}` tags that close a multi-line block without
/// repeating its name.
///
/// ## Why is this bad?
/// Once the opening tag has scrolled out of view, a bare `{% endblock %}` does not say which block
/// it closes, which makes long and nested blocks hard to follow. Django accepts the block's name as
/// a label on the closing tag and checks that it matches, and its template style guide asks for the
/// label whenever the closing tag is not on the same line as the opening one.
///
/// The label is decided from the layout at fix time: in a pre-commit hook that runs `check --fix`
/// before the formatter, a block the formatter then spreads over several lines gets its label on
/// the next run.
///
/// ## Example
/// ```html
/// {% block content %}
///     <h1>{{ title }}</h1>
/// {% endblock %}
/// ```
///
/// Use instead:
/// ```html
/// {% block content %}
///     <h1>{{ title }}</h1>
/// {% endblock content %}
/// ```
///
/// ## References
/// - [Django coding style: template style](https://docs.djangoproject.com/en/stable/internals/contributing/writing-code/coding-style/#template-style)
/// - [Django documentation: template inheritance](https://docs.djangoproject.com/en/stable/ref/templates/language/#template-inheritance)
/// - [Django documentation: template partials](https://docs.djangoproject.com/en/stable/ref/templates/language/#template-partials)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct MissingEndblockLabel<'a> {
    pub kind: BlockKind,
    pub name: &'a str,
}

impl Violation for MissingEndblockLabel<'_> {
    const RULE: Rule = Rule::MissingEndblockLabel;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        match self.kind {
            BlockKind::Block => "Missing label on `{% endblock %}` of a multi-line block".into(),
            BlockKind::Partialdef => {
                "Missing label on `{% endpartialdef %}` of a multi-line block".into()
            }
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some(
            format!(
                "Write it as `{{% {} {} %}}`",
                self.kind.end_tag(),
                self.name
            )
            .into(),
        )
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Add label")
    }
}

pub fn check(checker: &Checker<'_>, end: &BlockEnd<'_>) {
    if end.same_line || end.label.is_some() {
        return;
    }
    let mut guard = checker.report_diagnostic(
        &MissingEndblockLabel {
            kind: end.kind,
            name: end.name,
        },
        checker.source_span(end.tag_name),
    );
    guard.set_fix(Fix::safe_edit(Edit::insertion(
        [" ", end.name].concat(),
        checker.source_end(end.tag_name),
    )));
}

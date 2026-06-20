use markup_fmt::ast::{JinjaBlock, JinjaTagOrChildren, Node};

use crate::Checker;
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for multiple `{% block %}` tags that share the same name within a single template.
///
/// ## Why is this bad?
/// Django requires `{% block %}` names to be unique within a template. A block both provides a hole
/// for a child template to fill and defines the default content for that hole, so two blocks with
/// the same name are ambiguous. Django raises a `TemplateSyntaxError` when the template is parsed,
/// so a duplicate name breaks the template at runtime.
///
/// ## Example
/// ```html
/// {% block title %}Title 1{% endblock %}
/// {% block title %}Title 2{% endblock %}
/// ```
///
/// Use instead:
/// ```html
/// {% block title %}Title 1{% endblock %}
/// {% block subtitle %}Title 2{% endblock %}
/// ```
///
/// ## References
/// - [Django documentation: template inheritance](https://docs.djangoproject.com/en/stable/ref/templates/language/#template-inheritance)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.11")]
pub struct DuplicateBlockName<'a> {
    pub name: &'a str,
}

impl Violation for DuplicateBlockName<'_> {
    const RULE: Rule = Rule::DuplicateBlockName;
    const CATEGORY: RuleCategory = RuleCategory::Correctness;

    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Duplicate `{{% block %}}` name `{}`.", self.name)
    }

    fn help(&self) -> Option<String> {
        Some(format!(
            "Rename or remove one of the `{{% block {} %}}` tags; Django requires block names to be \
             unique within a template.",
            self.name
        ))
    }
}

pub fn block_name<'s>(block: &JinjaBlock<'s, Node<'s>>) -> Option<&'s str> {
    let Some(JinjaTagOrChildren::Tag(open_tag)) = block.body.first() else {
        return None;
    };
    // `{% block NAME %}`: one whitespace pass yields the tag (token 0) then the name (token 1).
    // Strip `{%-`/`{%+` markers first; otherwise they become a leading token and shift the name.
    let mut tokens = open_tag
        .content
        .trim_start_matches(['+', '-'])
        .split_ascii_whitespace();
    if tokens.next() != Some("block") {
        return None;
    }
    tokens.next()
}

/// Flag every block name that occurs more than once, reporting each occurrence after the first.
pub fn check(checker: &Checker<'_>) {
    let names = checker.block_names();
    for (i, &name) in names.iter().enumerate() {
        if names[..i].contains(&name) {
            let offset = checker.source_offset(name);
            checker.report_diagnostic(&DuplicateBlockName { name }, (offset, name.len()).into());
        }
    }
}

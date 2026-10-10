use std::borrow::Cow;

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
/// Django reads template tags before the HTML, so a block inside `<script>`, `<style>`, `<pre>` or
/// `<textarea>`, inside an HTML comment or an attribute value, or in attribute position counts like
/// any other.
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
    fn message(&self) -> Cow<'static, str> {
        format!("Duplicate `{{% block %}}` name `{}`", self.name).into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some(
            format!(
                "Rename or remove one of the `{{% block {} %}}` tags",
                self.name
            )
            .into(),
        )
    }
}

/// The name of a `{% block NAME %}` the caller recognised: the token after `block`.
/// Strip `{%-`/`{%+` markers first; otherwise they become a leading token and shift the name.
pub fn block_name_from_content(content: &str) -> Option<&str> {
    content
        .trim_start_matches(['+', '-'])
        .split_ascii_whitespace()
        .nth(1)
}

/// Flag every block name that occurs more than once, reporting each occurrence after the first.
pub fn check(checker: &Checker<'_>) {
    let names = checker.block_names();
    for (i, &name) in names.iter().enumerate() {
        if names[..i].contains(&name) {
            checker.report_diagnostic(&DuplicateBlockName { name }, checker.source_span(name));
        }
    }
}

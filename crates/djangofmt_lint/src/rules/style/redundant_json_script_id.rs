use std::borrow::Cow;

use markup_fmt::ast::JinjaInterpolation;

use crate::Checker;
use crate::django_version::DjangoVersion;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for the `json_script` filter called with an empty element id.
///
/// ## Why is this bad?
/// `json_script` renders an `id` attribute only for a non-empty element id, so `json_script:""`
/// emits exactly the same `<script>` tag as `json_script` on its own. The argument was mandatory
/// until Django 4.1 made it optional, and since then only adds noise.
///
/// The rule is version-gated: it reports nothing until `lint.target-version` names Django 4.1 or
/// newer, because dropping the argument is a template syntax error on older releases.
///
/// ## Example
/// ```html
/// {{ user_data|json_script:"" }}
/// ```
///
/// Use instead:
/// ```html
/// {{ user_data|json_script }}
/// ```
///
/// ## Options
/// - `lint.target-version`
///
/// ## References
/// - [Django template filters: `json_script`](https://docs.djangoproject.com/en/stable/ref/templates/builtins/#json-script)
/// - [Django 4.1 release notes](https://docs.djangoproject.com/en/stable/releases/4.1/#templates)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct RedundantJsonScriptId;

impl Violation for RedundantJsonScriptId {
    const RULE: Rule = Rule::RedundantJsonScriptId;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        "Redundant empty element id passed to `json_script`".into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some("Omit the argument".into())
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Remove the empty element id")
    }
}

/// The release that made `json_script`'s element id optional.
const ELEMENT_ID_OPTIONAL_IN: DjangoVersion = DjangoVersion::new(4, 1);

const JSON_SCRIPT_FILTER: &str = "|json_script";
const EMPTY_LITERALS: [&str; 2] = ["\"\"", "''"];

pub fn check(interpolation: &JinjaInterpolation<'_>, checker: &Checker<'_>) {
    if !checker.targets_django(ELEMENT_ID_OPTIONAL_IN) {
        return;
    }

    for argument in empty_element_ids(interpolation.expr) {
        let argument_span = checker.source_span(argument);
        let mut guard = checker.report_diagnostic(&RedundantJsonScriptId, argument_span);
        guard.set_fix(Fix::safe_edit(Edit::deletion(argument_span)));
    }
}

/// Each `:""` argument `expr` passes to a `json_script` filter.
///
/// Django forbids whitespace around `|` and `:`, so a filter and its argument are one unbroken
/// run and the argument ends where the expression, whitespace or the next filter begins.
fn empty_element_ids(expr: &str) -> impl Iterator<Item = &str> {
    expr.match_indices(JSON_SCRIPT_FILTER)
        .filter_map(move |(start, filter)| {
            let argument_start = start + filter.len();
            let argument = expr[argument_start..].strip_prefix(':')?;
            let literal = EMPTY_LITERALS
                .into_iter()
                .find(|literal| argument.starts_with(literal))?;
            if !argument[literal.len()..].starts_with(['|', ' ', '\t', '\n', '\r'])
                && argument.len() != literal.len()
            {
                return None;
            }
            Some(&expr[argument_start..argument_start + ":".len() + literal.len()])
        })
}

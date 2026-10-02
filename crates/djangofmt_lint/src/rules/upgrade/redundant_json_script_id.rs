use std::borrow::Cow;

use djangofmt_syntax::dtl::{filter_expression, is_space};
use djangofmt_syntax::span;
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
/// emits the same `<script>` tag as `json_script` on its own. The argument was mandatory until
/// Django 4.1 made it optional, and since then only adds noise.
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
/// - [Django 4.1 release notes: templates](https://docs.djangoproject.com/en/stable/releases/4.1/#templates-4-1)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct RedundantJsonScriptId;

impl Violation for RedundantJsonScriptId {
    const RULE: Rule = Rule::RedundantJsonScriptId;
    const CATEGORY: RuleCategory = RuleCategory::Upgrade;
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

/// The caller passes the target version, so files without one never get here.
pub fn check(
    checker: &Checker<'_>,
    interpolation: &JinjaInterpolation<'_>,
    target_version: DjangoVersion,
) {
    // Every `{{ }}` gets here, so only a filter argument's `:` sends it out of line.
    if checker.is_django()
        && target_version >= ELEMENT_ID_OPTIONAL_IN
        && interpolation.expr.contains(':')
    {
        check_filters(checker, interpolation.expr);
    }
}

#[cold]
fn check_filters(checker: &Checker<'_>, expr: &str) {
    // Django allows no whitespace between a filter and its `:`, so this skips lexing safely.
    if !expr.contains("json_script:") {
        return;
    }
    let Some(expression) = filter_expression(expr.trim_matches(is_space)) else {
        return;
    };
    for filter in expression.filters {
        if filter.name == "json_script"
            && let Some(arg @ ("\"\"" | "''")) = filter.arg
        {
            let start = checker.source_offset(arg) - ":".len();
            let argument = span(start, checker.source_end(arg) - start);
            checker
                .report_diagnostic(&RedundantJsonScriptId, argument)
                .set_fix(Fix::safe_edit(Edit::deletion(argument)));
        }
    }
}

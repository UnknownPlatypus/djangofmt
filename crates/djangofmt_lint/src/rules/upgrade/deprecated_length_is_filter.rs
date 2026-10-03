use std::borrow::Cow;

use djangofmt_syntax::dtl::{Filter, bits, filter_expression, first_bit_is};
use djangofmt_syntax::span;
use markup_fmt::ast::JinjaTag;

use crate::Checker;
use crate::django_version::DjangoVersion;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `{% if %}` and `{% elif %}` conditions that test a value with the `length_is` filter.
///
/// ## Why is this bad?
/// `length_is` was deprecated in Django 4.2 in favour of the `length` filter and the `==` operator,
/// and removed in Django 5.1, where a template that still uses it fails to compile.
///
/// The rewrite differs in two edge cases. `length_is` returns `len(value) == int(arg)`, or `""`
/// when either call fails, while `length` returns `0` for a value without a length: `length_is:0`
/// on a missing or unsized value was false, and `length == 0` is true. A variable argument is no
/// longer passed through `int()`: one holding the string `"3"` no longer matches a length of 3.
///
/// Only a condition made of the filtered value alone is reported. Other uses, such as in a longer
/// condition or in `{{ }}`, need rewriting by hand.
///
/// The rule is version-gated: it reports nothing until `lint.target-version` names Django 4.2 or
/// newer.
///
/// ## Example
/// ```html
/// {% if engines|length_is:1 %}One engine{% endif %}
/// ```
///
/// Use instead:
/// ```html
/// {% if engines|length == 1 %}One engine{% endif %}
/// ```
///
/// ## Options
/// - `lint.target-version`
///
/// ## References
/// - [Django 4.2 release notes: deprecated features](https://docs.djangoproject.com/en/stable/releases/4.2/#deprecated-features-4-2)
/// - [Django 5.1 release notes: removed features](https://docs.djangoproject.com/en/stable/releases/5.1/#features-removed-in-5-1)
/// - [Django template filters: `length`](https://docs.djangoproject.com/en/stable/ref/templates/builtins/#length)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct DeprecatedLengthIsFilter;

impl Violation for DeprecatedLengthIsFilter {
    const RULE: Rule = Rule::DeprecatedLengthIsFilter;
    const CATEGORY: RuleCategory = RuleCategory::Upgrade;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        "Deprecated `length_is` filter".into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some("Compare `length` with `==` instead".into())
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Compare `length` with `==`")
    }
}

/// The release that deprecated `length_is` in favour of `length` and `==`.
const DEPRECATED_IN: DjangoVersion = DjangoVersion::new(4, 2);

/// The caller passes the target version, so files without one never get here.
pub fn check(checker: &Checker<'_>, tag: &JinjaTag<'_>, target_version: DjangoVersion) {
    if !checker.is_django() || target_version < DEPRECATED_IN {
        return;
    }
    // Every tag gets here, so the name is matched without lexing and the rest runs out of line.
    if first_bit_is(tag.content, "if") || first_bit_is(tag.content, "elif") {
        check_condition(checker, tag.content);
    }
}

#[cold]
fn check_condition(checker: &Checker<'_>, content: &str) {
    // The filter must come last, so lexing waits until the last `|` opens `length_is:`.
    if !content
        .rsplit_once('|')
        .is_some_and(|(_, last)| last.starts_with("length_is:"))
    {
        return;
    }
    // Next to another comparison the new `==` would regroup, so only a lone value is rewritten.
    let [_, condition] = bits(content)[..] else {
        return;
    };
    // A filter after `length_is` reads its boolean, and `int()` made a string argument a number.
    if let Some(expression) = filter_expression(condition)
        && let Some(Filter {
            name: name @ "length_is",
            arg: Some(arg),
        }) = expression.filters.last()
        && !arg.contains(['"', '\''])
    {
        let start = checker.source_offset(name);
        let call = span(start, checker.source_end(arg) - start);
        checker
            .report_diagnostic(&DeprecatedLengthIsFilter, call)
            .set_fix(Fix::safe_edit(Edit::replacement(
                format!("length == {arg}"),
                call,
            )));
    }
}

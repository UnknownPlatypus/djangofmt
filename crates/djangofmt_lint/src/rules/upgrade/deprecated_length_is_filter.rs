use std::borrow::Cow;

use djangofmt_syntax::dtl::{Filter, bits, filter_expression};
use djangofmt_syntax::span;
use markup_fmt::ast::JinjaTag;

use crate::Checker;
use crate::django_version::DjangoVersion;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::IF_OPERATORS;
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
/// Only a condition made of the filtered value alone is reported; other uses, such as in a longer
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

/// The caller guarantees an `if` or `elif` and passes the target version, so files without one
/// never get here.
pub fn check(checker: &Checker<'_>, tag: &JinjaTag<'_>, target_version: DjangoVersion) {
    // Every `if` gets here, so nothing is lexed before the filter's name turns up.
    if checker.is_django() && target_version >= DEPRECATED_IN && tag.content.contains("length_is:")
    {
        check_condition(checker, tag.content);
    }
}

#[cold]
fn check_condition(checker: &Checker<'_>, content: &str) {
    // Next to another comparison the new `==` would regroup, so only a lone value is rewritten.
    let [_, condition] = bits(content)[..] else {
        return;
    };
    // A filter after `length_is` reads its boolean, which `length` does not return.
    if let Some(expression) = filter_expression(condition)
        && let Some(Filter {
            name: name @ "length_is",
            arg: Some(arg),
        }) = expression.filters.last()
        && let Some(operand) = operand(arg)
    {
        let start = checker.source_offset(name);
        let call = span(start, checker.source_end(arg) - start);
        checker
            .report_diagnostic(&DeprecatedLengthIsFilter, call)
            .set_fix(Fix::safe_edit(Edit::replacement(
                format!("length == {operand}"),
                call,
            )));
    }
}

/// The `==` operand matching `int(arg)`: a quoted integer drops its quotes, while another string
/// or an `{% if %}` operator word has none.
fn operand(arg: &str) -> Option<&str> {
    let Some(quoted) = arg.strip_prefix(['"', '\'']) else {
        return (!arg.contains(['"', '\'']) && !IF_OPERATORS.contains(&arg)).then_some(arg);
    };
    let number = quoted.strip_suffix(['"', '\''])?;
    (!number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())).then_some(number)
}

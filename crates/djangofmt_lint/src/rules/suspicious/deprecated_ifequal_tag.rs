use std::borrow::Cow;

use djangofmt_syntax::dtl::{bits, is_space};
use markup_fmt::ast::JinjaTag;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `{% ifequal %}` and `{% ifnotequal %}` blocks.
///
/// ## Why is this bad?
/// Both tags are deprecated since Django 3.1 and removed in Django 4.0, where a template that uses
/// them no longer compiles. `{% if %}` makes the same comparisons with `==` and `!=`.
///
/// The rule is version-gated: it reports nothing until `lint.target-version` names Django 3.1 or
/// newer.
///
/// ## Example
/// ```html
/// {% ifequal user.username "admin" %}
///     <a href="/admin/">Admin</a>
/// {% else %}
///     <a href="/account/">Account</a>
/// {% endifequal %}
/// ```
///
/// Use instead:
/// ```html
/// {% if user.username == "admin" %}
///     <a href="/admin/">Admin</a>
/// {% else %}
///     <a href="/account/">Account</a>
/// {% endif %}
/// ```
///
/// ## Options
/// - `lint.target-version`
///
/// ## References
/// - [Django 3.1 release notes: deprecated features](https://docs.djangoproject.com/en/stable/releases/3.1/#deprecated-features-3-1)
/// - [Django 4.0 release notes: removed features](https://docs.djangoproject.com/en/stable/releases/4.0/#features-removed-in-4-0)
/// - [Django template tags: `if`](https://docs.djangoproject.com/en/stable/ref/templates/builtins/#if)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct DeprecatedIfequalTag {
    pub tag: EqualityTag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqualityTag {
    Ifequal,
    Ifnotequal,
}

impl EqualityTag {
    /// The `{% if %}` operator that makes the same comparison.
    const fn operator(self) -> &'static str {
        match self {
            Self::Ifequal => "==",
            Self::Ifnotequal => "!=",
        }
    }
}

impl Violation for DeprecatedIfequalTag {
    const RULE: Rule = Rule::DeprecatedIfequalTag;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        match self.tag {
            EqualityTag::Ifequal => "Deprecated `{% ifequal %}` tag".into(),
            EqualityTag::Ifnotequal => "Deprecated `{% ifnotequal %}` tag".into(),
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some(format!("Use `{{% if %}}` with `{}` instead", self.tag.operator()).into())
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Replace with `{% if %}`")
    }
}

/// The bits `{% if %}` reads as operators rather than operands.
const IF_OPERATORS: [&str; 11] = [
    "and", "or", "not", "in", "is", "==", "!=", "<", ">", "<=", ">=",
];

/// The caller guarantees an `ifequal` or `ifnotequal` opener and the closer the token stream
/// pairs it with, wherever the HTML parser put either.
pub fn check(checker: &Checker<'_>, opener: &JinjaTag<'_>, closer: &JinjaTag<'_>) {
    let tag = if opener
        .content
        .trim_start_matches(is_space)
        .starts_with("ifnotequal")
    {
        EqualityTag::Ifnotequal
    } else {
        EqualityTag::Ifequal
    };
    // Django rejects any other arity.
    let [name, left, right] = bits(opener.content)[..] else {
        return;
    };
    // `{% ifequal in b %}` works, `{% if in == b %}` is a syntax error.
    if IF_OPERATORS.contains(&left) || IF_OPERATORS.contains(&right) {
        return;
    }
    let opener_edit = Edit::replacement(
        format!("if {left} {} {right}", tag.operator()),
        checker.source_span(opener.content.trim_matches(is_space)),
    );
    // Django never reads the closer's other bits.
    let closer_edit = Edit::replacement(
        "endif",
        checker.source_span(closer.content.trim_matches(is_space)),
    );
    checker
        .report_diagnostic(&DeprecatedIfequalTag { tag }, checker.source_span(name))
        .set_fix(Fix::safe_edits(opener_edit, [closer_edit]));
}

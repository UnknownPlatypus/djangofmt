use std::borrow::Cow;

use djangofmt_syntax::dtl::{bits, is_space};
use markup_fmt::ast::{JinjaBlock, JinjaTag, JinjaTagOrChildren};

use crate::Checker;
use crate::django_version::DjangoVersion;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::IF_OPERATORS;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `{% ifequal %}` and `{% ifnotequal %}` blocks.
///
/// ## Why is this bad?
/// Both tags are deprecated since Django 3.1 and removed in Django 4.0, where a template that uses
/// them no longer compiles. `{% if %}` makes the same comparisons with `==` and `!=`.
///
/// A block inside an attribute value, an HTML comment or a `<script>`, `<style>`, `<pre>` or
/// `<textarea>` body is not reported: djangofmt reads its opening and closing tags there as two
/// separate tags.
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
    const CATEGORY: RuleCategory = RuleCategory::Upgrade;
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

/// The release that deprecated both tags in favour of `{% if %}`.
const DEPRECATED_IN: DjangoVersion = DjangoVersion::new(3, 1);

/// The caller guarantees an `ifequal` or `ifnotequal` block, which `tag` names, and passes the
/// target version, so files without one never get here.
pub fn check<T>(
    checker: &Checker<'_>,
    block: &JinjaBlock<'_, T>,
    tag: EqualityTag,
    target_version: DjangoVersion,
) {
    if !checker.is_django() || target_version < DEPRECATED_IN {
        return;
    }
    let [
        JinjaTagOrChildren::Tag(opener),
        ..,
        JinjaTagOrChildren::Tag(closer),
    ] = &block.body[..]
    else {
        return;
    };
    check_pair(checker, opener, closer, tag);
}

#[cold]
fn check_pair(
    checker: &Checker<'_>,
    opener: &JinjaTag<'_>,
    closer: &JinjaTag<'_>,
    tag: EqualityTag,
) {
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

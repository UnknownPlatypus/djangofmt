use std::borrow::Cow;

use djangofmt_syntax::dtl::bits;
use markup_fmt::ast::JinjaTag;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `{% blocktranslate %}` / `{% blocktrans %}` blocks that omit
/// the `trimmed` option.
///
/// ## Why is this bad?
/// Without `trimmed`, indentation and whitespace inside the block become part
/// of the translation string in `.po` files. Reformatting the template then
/// reorders bytes inside translatable strings, producing noisy translation
/// diffs on every reformat.
///
/// ## Example
///
/// ```html
/// {% blocktranslate %}This string will have {{ value }} inside.{% endblocktranslate %}
/// ```
///
/// Use instead:
///
/// ```html
/// {% blocktranslate trimmed %}This string will have {{ value }} inside.{% endblocktranslate %}
/// ```
///
/// ## References
/// - [Django documentation: `blocktranslate`](https://docs.djangoproject.com/en/stable/topics/i18n/translation/#std-templatetag-blocktranslate)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "0.2.8")]
pub struct UntrimmedBlocktranslate;

impl Violation for UntrimmedBlocktranslate {
    const RULE: Rule = Rule::UntrimmedBlocktranslate;
    const CATEGORY: RuleCategory = RuleCategory::Correctness;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        "Missing `trimmed` on `{% blocktranslate %}`".into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some("Add `trimmed` to the opening tag".into())
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Add trimmed")
    }
}

/// The caller guarantees a `{% blocktranslate %}` or `{% blocktrans %}` opener and passes its
/// `name`, a slice of the opener.
pub fn check(checker: &Checker<'_>, open_tag: &JinjaTag<'_>, name: &str) {
    if bits(open_tag.content).contains(&"trimmed") {
        return;
    }
    checker
        .report_diagnostic(
            &UntrimmedBlocktranslate,
            checker.source_span(open_tag.content),
        )
        .set_fix(Fix::safe_edit(Edit::insertion(
            " trimmed",
            checker.source_end(name),
        )));
}

use markup_fmt::ast::{JinjaBlock, JinjaTagOrChildren, Node};
use markup_fmt::parser::parse_jinja_tag_name;

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
/// ## Fix safety
/// This rule's fix is marked as safe: it inserts ` trimmed` immediately after
/// the tag name in the opening tag without altering the translatable content.
///
/// ## References
/// - [Django documentation: `blocktranslate`](https://docs.djangoproject.com/en/stable/topics/i18n/translation/#std-templatetag-blocktranslate)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
pub struct UntrimmedBlocktranslate;

impl Violation for UntrimmedBlocktranslate {
    const RULE: Rule = Rule::UntrimmedBlocktranslate;
    const CATEGORY: RuleCategory = RuleCategory::Correctness;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> String {
        "`{% blocktranslate %}` should declare `trimmed` to avoid leaking \
         indentation into translation strings."
            .to_string()
    }

    fn help(&self) -> Option<String> {
        Some(
            "Add `trimmed` to the opening tag, e.g. \
             `{% blocktranslate trimmed %}...{% endblocktranslate %}`."
                .to_string(),
        )
    }

    fn fix_title(&self) -> Option<String> {
        Some("Add trimmed".to_string())
    }
}

/// Inspect the opening tag of a Jinja block and report a diagnostic if it
/// is a `blocktranslate` / `blocktrans` block missing the `trimmed` keyword.
pub fn check(block: &JinjaBlock<'_, Node<'_>>, checker: &Checker<'_>) {
    let Some(JinjaTagOrChildren::Tag(open_tag)) = block.body.first() else {
        return;
    };

    let tag_name = parse_jinja_tag_name(open_tag);
    if tag_name != "blocktranslate" && tag_name != "blocktrans" {
        return;
    }

    if open_tag
        .content
        .split_ascii_whitespace()
        .any(|w| w == "trimmed")
    {
        return;
    }

    let span = (open_tag.start, open_tag.content.len()).into();
    let Some(mut guard) = checker.report_diagnostic_if_enabled(&UntrimmedBlocktranslate, span)
    else {
        return;
    };

    let local_offset = open_tag
        .content
        .find(tag_name)
        .expect("tag_name was extracted from tag.content by parse_jinja_tag_name");
    let insertion_at = open_tag.start + local_offset + tag_name.len();
    guard.set_fix(Fix::safe_edit(Edit::insertion(" trimmed", insertion_at)));
}

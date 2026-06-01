use std::sync::LazyLock;

use markup_fmt::ast::NativeAttribute;
use rustywind_core::RustyWind;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

static SORTER: LazyLock<RustyWind> = LazyLock::new(RustyWind::default);

/// ## What it does
/// Checks for `class` attributes whose Tailwind CSS utility classes are not in the canonical
/// order produced by the Tailwind class sorter.
///
/// ## Why is this bad?
/// Tailwind recommends a single, deterministic class order so the same set of utilities always
/// appears the same way in the source. Sorting them automatically removes the effort of arranging
/// classes by hand and keeps diffs focused on real changes instead of reordering churn.
///
/// ## Example
/// ```html
/// <button class="text-white px-4 sm:px-8 py-2 sm:py-3 bg-sky-700 hover:bg-sky-800">...</button>
/// ```
///
/// Use instead:
/// ```html
/// <button class="bg-sky-700 px-4 py-2 text-white hover:bg-sky-800 sm:px-8 sm:py-3">...</button>
/// ```
///
/// ## References
/// - [Tailwind CSS: Automatic class sorting](https://tailwindcss.com/blog/automatic-class-sorting-with-prettier)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(preview_since = "NEXT_DJANGOFMT_VERSION")]
pub struct UnsortedTailwindClasses;

impl Violation for UnsortedTailwindClasses {
    const RULE: Rule = Rule::UnsortedTailwindClasses;
    const CATEGORY: RuleCategory = RuleCategory::Style;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> String {
        "CSS classes are not sorted in the canonical Tailwind order.".to_string()
    }

    fn fix_title(&self) -> Option<String> {
        Some("Sort Tailwind CSS classes".to_string())
    }
}

// The caller gates this on the `class` attribute name.
pub fn check(attr: &NativeAttribute<'_>, checker: &Checker<'_>) {
    let NativeAttribute {
        value: Some((value_str, offset)),
        ..
    } = attr
    else {
        return;
    };

    if value_str.trim_ascii().is_empty() {
        return;
    }

    if contains_interpolation(value_str) {
        return;
    }

    let sorted = SORTER.sort_classes(value_str);
    if sorted.as_str() == *value_str {
        return;
    }

    let span = (*offset, value_str.len()).into();
    let mut guard = checker.report_diagnostic(&UnsortedTailwindClasses, span);
    guard.set_fix(Fix::safe_edit(Edit::replacement(sorted, span)));
}

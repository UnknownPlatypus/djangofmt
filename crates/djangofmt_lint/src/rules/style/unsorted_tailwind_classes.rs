use std::borrow::Cow;

use markup_fmt::ast::NativeAttribute;
use rustywind_core::{RustyWind, SourceLanguage};

use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};
use crate::{Checker, span};

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
/// <button class="sm:py-3 text-white px-4 py-2 bg-sky-700 hover:bg-sky-800 sm:px-8">...</button>
/// ```
///
/// Use instead:
/// ```html
/// <button class="bg-sky-700 px-4 py-2 text-white hover:bg-sky-800 sm:px-8 sm:py-3">...</button>
/// ```
///
/// ## Fix safety
/// In addition to sorting classes, this rule will also remove duplicated ones.
///
/// ## Options
/// - `lint.unsorted-tailwind-classes.prefix`
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
    fn message(&self) -> Cow<'static, str> {
        "CSS classes are not sorted in the canonical Tailwind order.".into()
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Sort Tailwind CSS classes")
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

    let sorted = RustyWind {
        tailwind_prefix: checker
            .context()
            .settings()
            .unsorted_tailwind_classes
            .prefix
            .clone(),
        // Keep the original whitespace so multi-line class attributes retain their shape.
        preserve_whitespace: true,
        ..RustyWind::default()
    }
    .sort_class_value(value_str, SourceLanguage::Django);
    if sorted == *value_str {
        return;
    }

    let span = span(*offset, value_str.len());
    let mut guard = checker.report_diagnostic(&UnsortedTailwindClasses, span);
    guard.set_fix(Fix::safe_edit(Edit::replacement(sorted.into_owned(), span)));
}

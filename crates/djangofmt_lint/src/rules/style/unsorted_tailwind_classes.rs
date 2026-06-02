use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::LazyLock;

use markup_fmt::ast::NativeAttribute;
use rustywind_core::RustyWind;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

static SORTER: LazyLock<RustyWind> = LazyLock::new(RustyWind::default);

/// Cap on memoized `class` values per worker thread.
///
/// Sorting a class list is the expensive part of this rule, and real templates
/// reuse the same handful of `class` values many times over (e.g. a comparison
/// table repeats `icon icon-check` hundreds of times). The cap keeps memory
/// bounded on projects with a large number of distinct class lists; once it is
/// reached the cache is dropped and repopulated.
const SORT_CACHE_CAPACITY: usize = 8192;

thread_local! {
    /// Memoizes the canonical Tailwind ordering of a raw `class` value.
    ///
    /// Sorting is a pure, deterministic function of the input string, so cached
    /// results are reused across attributes and files handled by the same thread.
    static SORT_CACHE: RefCell<HashMap<Box<str>, Box<str>>> = RefCell::new(HashMap::new());
}

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

    fn help(&self) -> Option<String> {
        Some("Sort the classes into the canonical Tailwind order.".to_string())
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

    // Sorting is the costly part of this rule, so memoize it per raw value.
    // Templates reuse the same `class` strings many times, and the common case
    // (already-sorted) returns `None` without allocating; only an actual
    // violation clones the sorted string for the fix.
    let Some(sorted) = SORT_CACHE.with_borrow_mut(|cache| {
        if let Some(sorted) = cache.get(*value_str) {
            let sorted_ref: &str = sorted;
            return (sorted_ref != *value_str).then(|| sorted.clone());
        }

        let sorted: Box<str> = SORTER.sort_classes(value_str).into();
        let sorted_ref: &str = &sorted;
        let violation = (sorted_ref != *value_str).then(|| sorted.clone());

        if cache.len() >= SORT_CACHE_CAPACITY {
            cache.clear();
        }
        cache.insert((*value_str).into(), sorted);

        violation
    }) else {
        return;
    };

    let span = (*offset, value_str.len()).into();
    let mut guard = checker.report_diagnostic(&UnsortedTailwindClasses, span);
    guard.set_fix(Fix::safe_edit(Edit::replacement(sorted, span)));
}

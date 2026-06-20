use markup_fmt::ast::NativeAttribute;
use rustywind_core::RustyWind;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

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
/// ## Options
/// By default, utilities are matched against Tailwind's standard names. Projects using a custom
/// prefix must configure it, otherwise prefixed utilities are treated as unknown classes and left
/// unsorted:
///
/// ```toml
/// [tool.djangofmt.lint.unsorted-tailwind-classes]
/// prefix = "tw-"  # Tailwind v3; use "tw:" for v4
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

    // Built per call rather than cached: `RustyWind::default()` allocates nothing, and the
    // expensive sorter it drives (and any prefixed variant) is memoized globally inside rustywind.
    let sorted = RustyWind {
        tailwind_prefix: checker
            .context()
            .settings()
            .unsorted_tailwind_classes
            .prefix
            .clone(),
        ..RustyWind::default()
    }
    .sort_classes(value_str);
    if sorted.as_str() == *value_str {
        return;
    }

    let span = (*offset, value_str.len()).into();
    let mut guard = checker.report_diagnostic(&UnsortedTailwindClasses, span);
    guard.set_fix(Fix::safe_edit(Edit::replacement(sorted, span)));
}

#[cfg(test)]
mod tests {
    use markup_fmt::{Language, parser::Parser};

    use crate::settings::unsorted_tailwind_classes;
    use crate::{Applicability, Rule, RuleSet, Settings, check_ast, fix_ast};

    fn settings(prefix: Option<&str>) -> Settings {
        Settings {
            rules: RuleSet::from_rule(Rule::UnsortedTailwindClasses),
            unsorted_tailwind_classes: unsorted_tailwind_classes::Settings {
                prefix: prefix.map(str::to_owned),
            },
        }
    }

    /// With a configured prefix, v3-prefixed utilities are recognized and sorted into the same
    /// canonical order as their unprefixed equivalents.
    #[test]
    fn sorts_v3_prefixed_classes_when_prefix_configured() {
        let source = r#"<a class="tw-text-white tw-bg-blue-700 btn hover:tw-bg-blue-900 tw-mt-4">Sign up</a>"#;
        let mut parser = Parser::new(source, Language::Django, vec![]);
        let ast = parser.parse_root().unwrap();
        let settings = settings(Some("tw-"));

        assert_eq!(check_ast(source, &ast, &settings).len(), 1);
        assert_eq!(
            fix_ast(source, &ast, &settings, Applicability::Safe).output,
            r#"<a class="btn tw-mt-4 tw-bg-blue-700 tw-text-white hover:tw-bg-blue-900">Sign up</a>"#
        );
    }

    /// Without a configured prefix, those same v3-prefixed utilities are unknown classes and keep
    /// their authored order, so nothing is flagged.
    #[test]
    fn leaves_v3_prefixed_classes_unsorted_without_prefix() {
        let source = r#"<a class="tw-text-white tw-bg-blue-700 btn hover:tw-bg-blue-900 tw-mt-4">Sign up</a>"#;
        let mut parser = Parser::new(source, Language::Django, vec![]);
        let ast = parser.parse_root().unwrap();

        assert!(check_ast(source, &ast, &settings(None)).is_empty());
    }
}

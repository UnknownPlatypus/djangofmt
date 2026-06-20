use strum::IntoEnumIterator;

use crate::registry::Rule;
use crate::rule_selector::{RuleSelector, SelectionWarning};
use crate::rule_set::RuleSet;

/// Resolved configuration for the `unsorted-tailwind-classes` rule.
///
/// Ruff equivalent: a per-linter `settings::Settings` (e.g.
/// `flake8_boolean_trap::settings::Settings`) aggregated onto the linter settings. djangofmt keys
/// per-rule config by rule name since it has no plugin grouping.
pub mod unsorted_tailwind_classes {
    /// Settings for [`crate::registry::Rule::UnsortedTailwindClasses`].
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct Settings {
        /// Custom Tailwind prefix (e.g. `tw-` for v3, `tw:` for v4). `None` means no prefix.
        pub prefix: Option<String>,
    }
}

/// Configuration settings for the linter.
///
/// Ruff equivalent: `LinterSettings`.
#[derive(Debug, Clone)]
pub struct Settings {
    /// The set of rules that are active for this run.
    pub rules: RuleSet,
    /// Per-rule configuration for `unsorted-tailwind-classes`.
    pub unsorted_tailwind_classes: unsorted_tailwind_classes::Settings,
}

impl Default for Settings {
    /// The default selection: every stable rule, preview rules disabled.
    fn default() -> Self {
        RuleSelection::default().into_settings().0
    }
}

impl Settings {
    /// Every runnable rule, including preview rules.
    #[must_use]
    pub fn all() -> Self {
        Self {
            rules: Rule::iter()
                .filter(|rule| !rule.is_deprecated() && !rule.is_removed())
                .collect(),
            unsorted_tailwind_classes: unsorted_tailwind_classes::Settings::default(),
        }
    }

    /// Check if a specific rule is enabled.
    #[must_use]
    #[inline]
    pub const fn is_enabled(&self, rule: Rule) -> bool {
        self.rules.contains(rule)
    }

    /// Check if any of the given rules is enabled.
    #[must_use]
    #[inline]
    pub const fn any_rule_enabled(&self, rules: &[Rule]) -> bool {
        let mut i = 0;
        while i < rules.len() {
            if self.is_enabled(rules[i]) {
                return true;
            }
            i += 1;
        }
        false
    }
}

/// Raw lint input, merged from CLI flags and `[tool.djangofmt.lint]`, resolved into a [`Settings`]
/// by [`RuleSelection::into_settings`]. Carries both rule selection and per-rule configuration.
///
/// Ruff equivalent: `LintConfiguration`.
#[derive(Debug, Clone, Default)]
pub struct RuleSelection {
    /// Selectors to enable. `None` falls back to the default selection (`category:all`).
    pub select: Option<Vec<RuleSelector>>,
    /// Selectors to disable.
    pub ignore: Vec<RuleSelector>,
    /// Whether preview rules are enabled.
    pub preview: bool,
    /// Resolved per-rule config forwarded to [`Settings::unsorted_tailwind_classes`].
    pub unsorted_tailwind_classes: unsorted_tailwind_classes::Settings,
}

impl RuleSelection {
    /// Resolve the selection into a [`Settings`] plus any non-fatal warnings.
    ///
    /// Selectors are applied in ascending specificity (`all` < category < rule),
    /// so a more specific selector always wins over a broader one regardless of order.
    /// On an equal-specificity tie, `ignore` wins over `select`.
    #[must_use]
    pub fn into_settings(self) -> (Settings, Vec<SelectionWarning>) {
        let select = self.select.unwrap_or_else(|| vec![RuleSelector::All]);
        let preview = self.preview;

        let mut items: Vec<(RuleSelector, bool)> = select
            .into_iter()
            .map(|selector| (selector, true))
            .chain(self.ignore.into_iter().map(|selector| (selector, false)))
            .collect();
        items.sort_by_key(|(selector, is_select)| {
            (selector.specificity(), u8::from(!*is_select), *selector)
        });
        items.dedup();

        let mut rules = RuleSet::default();
        let mut warnings = Vec::new();
        for (selector, is_select) in items {
            if is_select && selector.is_exact() {
                for rule in selector.all_rules() {
                    if rule.is_removed() {
                        warnings.push(SelectionWarning::RemovedRuleSelected(rule));
                    } else if rule.is_preview() && !preview {
                        warnings.push(SelectionWarning::PreviewRuleSkipped(rule));
                    } else if rule.is_deprecated() {
                        warnings.push(SelectionWarning::DeprecatedRuleSelected(rule));
                    }
                }
            }

            for rule in selector.rules(preview) {
                if is_select {
                    rules.insert(rule);
                } else {
                    rules.remove(rule);
                }
            }
        }

        (
            Settings {
                rules,
                unsorted_tailwind_classes: self.unsorted_tailwind_classes,
            },
            warnings,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use strum::VariantNames;

    use super::{RuleSelection, Settings, unsorted_tailwind_classes};
    use crate::registry::{Rule, RuleCategory};
    use crate::rule_selector::{RuleSelector, SelectionWarning};
    use crate::rule_set::RuleSet;

    #[test]
    fn any_rule_enabled_reflects_membership() {
        let all = Settings::default();
        assert!(all.any_rule_enabled(&[Rule::UseHttps]));
        assert!(all.any_rule_enabled(&[Rule::UseHttps, Rule::InvalidAttrValue]));
        assert!(!all.any_rule_enabled(&[]));

        let none = Settings {
            rules: RuleSet::default(),
            unsorted_tailwind_classes: unsorted_tailwind_classes::Settings::default(),
        };
        assert!(!none.any_rule_enabled(&[Rule::UseHttps, Rule::InvalidAttrValue]));

        let partial = Settings {
            rules: RuleSet::from_rule(Rule::UseHttps),
            unsorted_tailwind_classes: unsorted_tailwind_classes::Settings::default(),
        };
        assert!(partial.any_rule_enabled(&[Rule::UseHttps, Rule::InvalidAttrValue]));
        assert!(!partial.any_rule_enabled(&[Rule::InvalidAttrValue]));
    }

    /// a bare rule in `select` (specificity 2) beats a `category:` in `ignore` (specificity 1), which beats `category:all` (specificity 0).
    #[test]
    fn specificity_resolution_is_order_independent() {
        let selection = RuleSelection {
            select: Some(vec![RuleSelector::All, RuleSelector::Rule(Rule::UseHttps)]),
            ignore: vec![RuleSelector::Category(RuleCategory::Suspicious)],
            preview: false,
            unsorted_tailwind_classes: unsorted_tailwind_classes::Settings::default(),
        };
        let (settings, warnings) = selection.into_settings();
        assert!(warnings.is_empty());

        // The whole suspicious category is off...
        assert!(!settings.is_enabled(Rule::JavascriptUrl));
        assert!(!settings.is_enabled(Rule::DuplicateAttr));
        // ...except use-https, re-enabled by the more specific bare selector.
        assert!(settings.is_enabled(Rule::UseHttps));
        // Other stable categories remain on.
        assert!(settings.is_enabled(Rule::InvalidAttrValue));
    }

    #[test]
    fn default_excludes_preview_but_all_includes_it() {
        let default = Settings::default();
        assert!(!default.is_enabled(Rule::EmptyTagPair)); // preview rule
        assert!(default.is_enabled(Rule::InvalidAttrValue));

        let all = Settings::all();
        assert!(all.is_enabled(Rule::EmptyTagPair));
        assert!(all.is_enabled(Rule::InvalidAttrValue));
    }

    #[test]
    fn preview_gate_for_category_and_explicit_rule() {
        // category:all with preview on includes preview rules.
        let all_rules = RuleSelection {
            preview: true,
            ..RuleSelection::default()
        }
        .into_settings()
        .0;
        assert!(all_rules.is_enabled(Rule::EmptyTagPair));

        // Naming a preview rule without preview warns and skips it.
        let (settings, warnings) = RuleSelection {
            select: Some(vec![RuleSelector::Rule(Rule::EmptyTagPair)]),
            ..RuleSelection::default()
        }
        .into_settings();
        assert!(!settings.is_enabled(Rule::EmptyTagPair));
        assert_eq!(
            warnings,
            vec![SelectionWarning::PreviewRuleSkipped(Rule::EmptyTagPair)]
        );
    }

    /// A selector listed twice is collapsed even when another selector sits between the duplicates
    #[test]
    fn duplicate_selectors_are_deduped() {
        let (settings, warnings) = RuleSelection {
            select: Some(vec![
                RuleSelector::Rule(Rule::EmptyTagPair),
                RuleSelector::Rule(Rule::UseHttps),
                RuleSelector::Rule(Rule::EmptyTagPair),
            ]),
            ..RuleSelection::default()
        }
        .into_settings();

        assert!(settings.is_enabled(Rule::UseHttps));
        assert!(!settings.is_enabled(Rule::EmptyTagPair));
        assert_eq!(
            warnings,
            vec![SelectionWarning::PreviewRuleSkipped(Rule::EmptyTagPair)]
        );
    }

    #[test]
    fn no_rule_name_collides_with_a_group() {
        for group in std::iter::once("all").chain(RuleCategory::VARIANTS.iter().copied()) {
            assert!(
                Rule::from_str(group).is_err(),
                "category/group `{group}` collides with a rule name"
            );
        }
    }
}

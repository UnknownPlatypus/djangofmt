//! Resolver tests for `Settings::from_selectors`.
//!
//! Exercises the specificity-ordered layered resolver against the existing
//! Stable rule (`invalid-attr-value`). Preview-only behavior is asserted at
//! the unit-test level on `RuleSelector::rules` (no preview-group rules exist
//! yet); once a `Preview` rule is added in a follow-up PR, those scenarios
//! can be promoted to full resolver tests here.

use djangofmt_lint::{PreviewMode, Rule, RuleCategory, RuleSelector, Settings};

const fn category(c: RuleCategory) -> RuleSelector {
    RuleSelector::Category(c)
}

const fn rule(r: Rule) -> RuleSelector {
    RuleSelector::Rule(r)
}

#[test]
fn default_enables_invalid_attr_value() {
    let settings = Settings::default();
    assert!(settings.is_enabled(Rule::InvalidAttrValue));
    assert_eq!(settings.preview, PreviewMode::Disabled);
}

#[test]
fn select_all_includes_stable_rules() {
    let settings =
        Settings::from_selectors(&[RuleSelector::All], &[], &[], &[], PreviewMode::Disabled);
    assert!(settings.is_enabled(Rule::InvalidAttrValue));
}

#[test]
fn empty_select_disables_everything() {
    let settings = Settings::from_selectors(&[], &[], &[], &[], PreviewMode::Disabled);
    assert!(!settings.is_enabled(Rule::InvalidAttrValue));
    assert!(settings.rules.is_empty());
}

#[test]
fn exact_ignore_beats_all_select() {
    let settings = Settings::from_selectors(
        &[RuleSelector::All],
        &[rule(Rule::InvalidAttrValue)],
        &[],
        &[],
        PreviewMode::Disabled,
    );
    assert!(!settings.is_enabled(Rule::InvalidAttrValue));
}

#[test]
fn exact_select_beats_category_ignore() {
    // Even though `correctness` is ignored at the category level, the exact
    // `invalid-attr-value` selector is more specific and should re-enable it.
    let settings = Settings::from_selectors(
        &[rule(Rule::InvalidAttrValue)],
        &[category(RuleCategory::Correctness)],
        &[],
        &[],
        PreviewMode::Disabled,
    );
    assert!(settings.is_enabled(Rule::InvalidAttrValue));
}

#[test]
fn category_select_enables_matching_rules() {
    let settings = Settings::from_selectors(
        &[category(RuleCategory::Correctness)],
        &[],
        &[],
        &[],
        PreviewMode::Disabled,
    );
    assert!(settings.is_enabled(Rule::InvalidAttrValue));
}

#[test]
fn category_select_for_unmatched_category_enables_nothing() {
    let settings = Settings::from_selectors(
        &[category(RuleCategory::Style)],
        &[],
        &[],
        &[],
        PreviewMode::Disabled,
    );
    assert!(!settings.is_enabled(Rule::InvalidAttrValue));
    assert!(settings.rules.is_empty());
}

#[test]
fn extend_select_adds_on_top_of_select() {
    // select=[Style] enables nothing today; extend_select=[Correctness] adds
    // invalid-attr-value on top.
    let settings = Settings::from_selectors(
        &[category(RuleCategory::Style)],
        &[],
        &[category(RuleCategory::Correctness)],
        &[],
        PreviewMode::Disabled,
    );
    assert!(settings.is_enabled(Rule::InvalidAttrValue));
}

#[test]
fn extend_ignore_subtracts_after_select() {
    let settings = Settings::from_selectors(
        &[RuleSelector::All],
        &[],
        &[],
        &[rule(Rule::InvalidAttrValue)],
        PreviewMode::Disabled,
    );
    assert!(!settings.is_enabled(Rule::InvalidAttrValue));
}

#[test]
fn preview_mode_field_propagated() {
    let settings = Settings::from_selectors(&[], &[], &[], &[], PreviewMode::Enabled);
    assert_eq!(settings.preview, PreviewMode::Enabled);
}

#[test]
fn preview_mode_from_bool() {
    assert_eq!(PreviewMode::from(true), PreviewMode::Enabled);
    assert_eq!(PreviewMode::from(false), PreviewMode::Disabled);
}

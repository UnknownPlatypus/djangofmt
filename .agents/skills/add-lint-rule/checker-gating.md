# Gating rules in the checker

Disclosed reference for `add-lint-rule` Step 4. The default wiring — one `is_rule_enabled(Rule::MyRule)` guard per rule — is all most rules need. Reach here only when several rules share expensive work or key off the same value.

## Cluster-gate shared expensive work with `any_rule_enabled`

`checker.any_rule_enabled(&[Rule::A, Rule::B, ...])` returns true if *any* listed rule is enabled (a cheap bitset OR). It's worth using **only** to guard a block that does **expensive shared work** before the per-rule checks — work you'd rather not pay when none of the cluster is on. The canonical case gates a helper that walks the subtree / builds state shared by several rules:

```rust
if checker.any_rule_enabled(&[
    Rule::SuperfluousElseReturn,
    Rule::SuperfluousElseRaise,
    // ...
]) {
    superfluous_elif_else(checker, &stack); // expensive: builds `stack`, walks the body
}
```

Do **not** wrap cheap per-rule dispatch in `any_rule_enabled` just to group rules. With the default (all rules enabled) the gate is always true, so it only adds a test and never skips — benchmarked on a ~2.9k-template project it made no measurable difference.

## Classify once, then dispatch mutually-exclusive rules

If a cluster of rules is **mutually exclusive** (e.g. each fires on a single tag), classify the discriminating value once and dispatch, rather than calling every rule and having each re-check the same precondition:

```rust
// in visit_element: an element is at most one tag, so classify once
let tag = element.tag_name;
if tag.eq_ignore_ascii_case("form") {
    if self.is_rule_enabled(Rule::UppercaseFormMethod) { /* ... */ }
    // ... other form rules
} else if tag.eq_ignore_ascii_case("img") {
    // ... img rules
}
```

When you lift such a precondition into the dispatcher, delete the now-redundant guard from the rule's `check` and record the contract on it (e.g. `/// The caller guarantees`element`is a`<form>`.`).

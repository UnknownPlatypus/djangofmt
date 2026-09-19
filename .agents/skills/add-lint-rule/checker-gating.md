# Gating rules in the checker

Disclosed reference for `add-lint-rule` Step 4. The default wiring — one `is_rule_enabled(Rule::MyRule)` guard per rule — is all most rules need. Reach here when several rules key off the same value.

## Classify once, then dispatch mutually-exclusive rules

If a cluster of rules is **mutually exclusive** (e.g. each fires on a single tag), classify the discriminating value once and dispatch, rather than calling every rule and having each re-check the same precondition. `visit_element` already does this for `img`, `html`, `head` and `th`:

```rust
// an element is at most one tag, so classify once
if element.tag_name.eq_ignore_ascii_case("img") {
    if self.is_rule_enabled(Rule::MissingImgAlt) { /* ... */ }
    // ... other img rules
} else if element.tag_name.eq_ignore_ascii_case("html") {
    // ... html rules
}
```

When you lift such a precondition into the dispatcher, delete the now-redundant guard from the rule's `check` and record the contract in its doc comment (e.g. `/// The caller guarantees the element is an <img>.`).

`checker.any_rule_enabled(&[Rule::A, Rule::B])` exists for a cluster that shares expensive setup (a subtree walk, a built index) before its per-rule checks. Around cheap dispatch it skips nothing: with every rule enabled, the default, the gate is always true.

---
name: add-lint-rule
description: End-to-end process for a lint rule in djangofmt_lint — scoping, implementation, fixtures, real-template smoke test, single-commit landing. Use when adding, completing, or porting a lint rule.
---

# Add Lint Rule

## Before starting

Pin down the rule's **full scope** — *every* distinct check it must perform — before writing code. A rule that maps to a spec but implements only its obvious half ships broken.

- **Rule maps to a spec or accessibility technique?** Treat that source's **test procedure** as the checklist and implement every step of it. WCAG technique H63, for instance, is two checks — every `<th>` has a `scope` *and* every `scope` value is one of `row`/`col`/`rowgroup`/`colgroup`; a rule that only checks presence is half a rule. Cross-check how established linters (axe-core, Nu HTML Checker, eslint-plugin-jsx-a11y) cover the same concern to surface dimensions you'd otherwise miss — and keep concerns *they* split into separate rules separate (scope-on-a-non-`<th>` is its own rule, not part of this one).
- **Rule ported from another linter?** Its tests and issue tracker hold the edge cases that bit real users.

Both porting research and grounding fixtures in real code have exact mechanics — see **[research.md](research.md)**.

## Step 1: Create the rule file

Create `crates/djangofmt_lint/src/rules/{category}/{rule_name}.rs`.

Categories map to `RuleCategory`: `correctness`, `suspicious`, `style`, `complexity`, `accessibility`, `nursery`.

The file must contain:

### 1a. The violation struct with doc comment

The doc comment on the struct **is** the rule's documentation and its single source of truth. It MUST be one `///` doc comment attached directly to the struct — not a `//!` module-level comment, and not a second `///` block elsewhere in the file. The `#[derive(ViolationMetadata)]` macro (step 1b) reads it from the struct to populate `docs/rules/{name}.md`.

Write it for the template author reading the rendered docs: **what** the rule flags and why, NEVER **how** the implementation detects it. Detection mechanics — path matching, gates, skip conditions — live in the code; in the docs they are noise that goes stale.

Mirror the structure of `RedundantTypeAttr` exactly:

````rust
/// ## What it does
/// Checks for X.
///
/// ## Why is this bad?
/// Explanation of why this pattern is problematic.
///
/// Optional follow-up paragraph documenting exclusions or edge cases that
/// callers should know about (interpolated values, case sensitivity, etc.).
///
/// ## Example
/// ```html
/// <form method="put"></form>
/// ```
///
/// Use instead:
/// ```html
/// <form method="post"></form>
/// ```
///
/// ## Fix safety
/// (Only for rules that produce a fix.) State whether the fix is safe or
/// unsafe and why — e.g. "marked as safe: removing the attribute preserves
/// runtime semantics."
///
/// ## References
/// - [Spec/docs link](https://example.com/relevant-spec)
#[derive(Debug, PartialEq, Eq)]
pub struct MyRule {
    // Fields used in message() and help()
}
````

Formatting rules (these match `RedundantTypeAttr` exactly — diverging is an error):

- **Line width**: wrap prose at column 100 (the workspace `rustfmt` width), counting the `///` prefix. Do not wrap earlier — short 60–80 column lines waste vertical space and produce noisy diffs when neighbouring text is edited.
- `## What it does` — one sentence, starts with "Checks for". Content on the line **immediately after** the heading, no blank `///` line in between.
- `## Why is this bad?` — content immediately after the heading. Add follow-up paragraphs (separated by blank `///`) for exclusions or non-obvious behaviour. Keep the voice declarative and plain ("`eval()` is insecure as it enables arbitrary code execution"). Avoid editorial flair like "classic X sink" or "brittle across browsers", filler adjectives, and second-person ("you").
- `## Example` — code fence on the line **immediately after** the heading. No blank `///` between heading and `` ```html ``. After the closing fence, blank line, then plain text `Use instead:` (NOT a sub-heading, no `##`), then the corrected code fence immediately on the next line. Use HTML/Jinja, not Python.
- `## Fix safety` — include only when the rule registers a fix. One short paragraph documenting the safety classification and why.
- `## References` — include when there is a relevant spec, framework doc, or upstream issue to link. Bullet list, one link per line. Link primary sources only (WHATWG/W3C specs, MDN, framework documentation, CWE/OWASP). Do **not** link other linters' rule pages; the cross-reference belongs in the PR description, not in the user-facing docs.

### 1b. Derive `ViolationMetadata` and declare the lifecycle

Add `ViolationMetadata` to the struct's `#[derive(...)]` list, then add a `#[violation_metadata(...)]` attribute declaring the rule's lifecycle status. The derive captures the doc comment above and records the source file/line; the attribute records which version introduced the rule and whether it ships on by default. Both feed the `djangofmt_dev` generator that builds `docs/rules/{name}.md` and `docs/rules.md`.

```rust
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct MyRule {
    // Fields used in message() and help()
}
```

**Always use the `NEXT_DJANGOFMT_VERSION` placeholder for the version — never a hardcoded number.** `release.sh` rewrites every `NEXT_DJANGOFMT_VERSION` occurrence under `crates/djangofmt_lint/src/rules` to the real version at release time, so a new rule cannot know which release it will ship in. Hardcoding a version (e.g. `"0.2.9"`) is wrong and bypasses the release tooling — the existing rules show literal versions only because a past release already stamped them.

Pick the lifecycle keyword for the rule:

- `stable_since` — enabled by default. Use for clear, low-false-positive rules.
- `preview_since` — only runs in preview mode. Use while a new or heuristic rule is validated against real templates, so it is not on by default until proven.

(`deprecated_since` / `removed_since` also exist, for retiring a rule.)

Import the trait alongside `Violation`:

```rust
use std::borrow::Cow;

use crate::violation::{Violation, ViolationMetadata};
```

### 1c. Implement `Violation`

```rust
impl Violation for MyRule {
    const RULE: Rule = Rule::MyRule;
    const CATEGORY: RuleCategory = RuleCategory::Style; // pick the right one
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always; // omit if no fix

    fn message(&self) -> Cow<'static, str> {
        // Concise, includes relevant values from fields.
        // Static text: `"…".into()`; formatted: `format!("…", …).into()`.
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        // Actionable fix suggestion, or None
    }

    // Only for rules that produce a fix:
    fn fix_title(&self) -> Option<&'static str> {
        Some("Short imperative summary")
    }
}
```

`FIX_AVAILABILITY` defaults to `FixAvailability::None`; omit it for fixless rules. Use `FixAvailability::Always` when every diagnostic carries a fix, `FixAvailability::Sometimes` when the fix is conditional.

**More than one failure mode?** When a rule flags several distinct problems — a `scope` that is *missing* vs. one with an *invalid value* — keep it a single rule and store the modes as an inner enum on the struct; `message()` and `help()` then `match` on the variant. `#[derive_message_formats]` supports `match` arms, so each mode gets its own message and help:

```rust
pub enum ScopeViolation {
    MissingOrEmpty,
    InvalidValue { value: String },
}
pub struct TableHeaderMissingScope {
    pub kind: ScopeViolation,
}
```

Reference: `MissingTitle` / `TitleViolation` (`accessibility/missing_title.rs`).

### 1d. The check function

```rust
pub fn check(element: &Element<'_>, checker: &Checker<'_>) {
    // Guard: return early if element/attr doesn't match
    // Skip interpolated values with helpers::contains_interpolation()
    let mut guard = checker.report_diagnostic(&violation, span);
    // For rules with fixes, attach via the guard before it drops:
    guard.set_fix(Fix::safe_edit(Edit::deletion(span)));
}
```

Key points:

- The `Checker` is passed as `&Checker<'_>`, **not** `&mut` — diagnostics are buffered through interior mutability (`RefCell`).
- `checker.report_diagnostic(&violation, span)` returns a `DiagnosticGuard`. On `Drop` the guard pushes the diagnostic into the context's buffer. Hold the guard in a `let mut guard = ...` binding only if you need to attach a fix or override fields; otherwise let the temporary drop immediately.
- If the rule is **not** gated upfront in `checker.rs` (Step 4), call `checker.report_diagnostic_if_enabled(...)` instead — it returns `Option<DiagnosticGuard>` and short-circuits when disabled.
- For fixes: build an `Edit` (`Edit::deletion`, `Edit::insertion`, `Edit::replacement`) and wrap it with `Fix::safe_edit(...)` or `Fix::unsafe_edit(...)`, then call `guard.set_fix(fix)`.
- Rules that need the raw source offset of an AST slice use `checker.source_offset(slice)`.
- **Report the narrowest span that names the problem** — the offending value or attribute, not the whole element. Each failure mode can point at its own slice: `source_offset(value)` for a bad value, `source_offset(attr.name)` for a bad attribute, `source_offset(element.tag_name)` when the element itself is at fault.
- **Match HTML case-insensitively** — tag names, attribute names, and enumerated values alike: `name.eq_ignore_ascii_case("scope")`, `value.eq_ignore_ascii_case("col")`.
- For attribute rules, fold the value match into the `let…else` that destructures the attribute rather than unwrapping it in a separate step: `let Attribute::Native(NativeAttribute { name, value: Some((value_str, offset)), .. }) = attr else { continue; };`.

The function signature depends on what AST node the rule inspects. Element-level rules take `&Element<'_>`; Jinja block rules take `&JinjaBlock<'_, Node<'_>>`.

## Step 2: Export the module

Add `pub mod rule_name;` to `crates/djangofmt_lint/src/rules/{category}/mod.rs`.

If the category module doesn't exist yet, create `mod.rs` with `pub mod {rule_name};` and add `pub mod {category};` to `crates/djangofmt_lint/src/rules/mod.rs`.

## Step 3: Register the rule

Add an entry to the `define_rules!` macro in `crates/djangofmt_lint/src/registry.rs`:

```rust
define_rules! {
    (InvalidAttrValue, rules::correctness::invalid_attr_value::InvalidAttrValue),
    (MyRule, rules::style::my_rule::MyRule),  // <-- add here
}
```

The compiler verifies the violation struct exists and its `RULE` constant matches.

## Step 4: Wire it in the checker

Add the rule check call in the appropriate `visit_*` method in `crates/djangofmt_lint/src/checker.rs`, gated by `is_rule_enabled`:

```rust
fn visit_element(&mut self, element: &Element<'_>) {
    if self.is_rule_enabled(Rule::MyRule) {
        rules::style::my_rule::check(element, self);
    }
    // ...
}
```

If the rule's `check` uses `report_diagnostic_if_enabled` internally instead of being gated here, you can skip the `is_rule_enabled` wrapper — but gating upfront is cheaper when the rule does any non-trivial work before reporting.

**Sharing expensive setup across a cluster of rules, or dispatching mutually-exclusive rules off one tag?** See **[checker-gating.md](checker-gating.md)** for `any_rule_enabled` and classify-once dispatch.

## Step 5: Create test fixtures

Create directory `crates/djangofmt_lint/tests/check/{rule_name}/` with two files.
The directory name must be the rule code with underscores — the test runner derives
the rule from it and runs **only that rule**, so fixtures never trip other rules'
diagnostics (and a coverage test fails if the directory is missing or misnamed):

### `{rule_name}.invalid.html`

Contains **only** cases that should produce diagnostics. Every line/block that looks like it might be valid should be in the valid file instead. Group cases with HTML comments (`<!-- Case-insensitive tag name -->`) — the snapshot becomes much easier to review.

### `{rule_name}.valid.html`

Contains cases that must produce **zero** diagnostics. This file is critical — it catches false positives. Include:

- The obvious correct usage
- Edge cases: interpolated values (`{{ var }}`, `{% if %}`), missing attributes, empty values
- Elements that look similar but shouldn't trigger (e.g., `<div method="put">` for a form-only rule)
- Boundary cases from the original linter's test suite if porting
- **Every false-positive case and every rejected "extend the rule to X" feature request found in the upstream issue tracker**, each anchored with the inline `Regression:` citation comment specified in **[research.md](research.md)** so the case can't be silently removed later.

### Porting or grounding in real code

Adapt the issue-derived and Sourcegraph cases from **[research.md](research.md)** into HTML/Jinja, each with its inline citation comment. Don't drop edge cases — if the original tests something, there's a reason.

## Step 6: Run tests and accept snapshots

```bash
cargo test -p djangofmt_lint
cargo insta accept
cargo test -p djangofmt_lint  # verify everything passes
```

Snapshots auto-generated by the test runner in `tests/check/main.rs`:

- `{rule_name}.invalid.snap` — rendered diagnostics for the invalid fixture.
- `{rule_name}.invalid.fixed.snap` — post-fix source, generated only when the rule attaches a safe `Fix`.

Review both. The diagnostic messages, spans, help text, and the fixed source should all make sense; in particular check that the fix doesn't leave orphan whitespace or break surrounding markup.

## Step 7: Smoke-test against real templates

Run the new rule over `~/greenday`, a large real-world template corpus, and scan the output for false positives:

```bash
cargo run -p djangofmt -- check --select {rule-slug} ~/greenday
```

Add `--preview` if the rule is `preview_since`. If a real template surfaces a legitimate pattern the rule shouldn't flag, move it into `{rule_name}.valid.html` and teach the rule to skip it.

## Step 8: Generate and proofread the rule documentation

```bash
just docs-generate
```

This refreshes `docs/rules.md` and writes `docs/rules/{rule_name}.md` from the violation struct's doc comment. The output is gitignored (`docs/.gitignore`) — commit nothing; the step exists to prove the generator accepts the doc comment and to proofread the rendered page.

## Step 9: Commit as a single commit

Run `just pre-mr-check` and get it green first.

A new rule lands as **one** commit on a branch, not a stack of "add", "fix tests", "simplify", "doc tweak" commits. Squash any review follow-ups into that single commit before pushing.

**Title**: ``feat(lint): Add `{rule-slug}` lint rule`` — the `feat(lint):` conventional-commit prefix, then backticks around the kebab-case slug (e.g. ``feat(lint): Add `javascript-url` lint rule``).

**Body**: the rule's docstring verbatim, `///` prefix stripped — section headings (`## What it does` through `## References`), prose, and code fences exactly as written in the source. This duplicates the doc comment into the commit message on purpose: the body is what reviewers and `git log` readers see, and it is the single best place to capture the rationale at the time of landing.

## Reference files

- Reference rule with safe fix (doc layout, `DiagnosticGuard`, `Fix::safe_edit`): `crates/djangofmt_lint/src/rules/style/redundant_type_attr.rs`
- Reference rule without fix: `crates/djangofmt_lint/src/rules/correctness/invalid_attr_value.rs`
- Multi-variant (enum) violation with `match` in `message()`/`help()`: `crates/djangofmt_lint/src/rules/accessibility/missing_title.rs`
- Full-technique rule (WCAG H63) — multi-variant, per-variant spans, case-insensitive value validation: `crates/djangofmt_lint/src/rules/accessibility/table_header_missing_scope.rs`
- Jinja-block-shaped rule with fix: `crates/djangofmt_lint/src/rules/correctness/untrimmed_blocktranslate.rs`
- Violation trait: `crates/djangofmt_lint/src/violation.rs`
- `LintContext` / `DiagnosticGuard`: `crates/djangofmt_lint/src/lint_context.rs`
- Fix data model (`Edit`, `Fix`, `Applicability`, `FixAvailability`): `crates/djangofmt_lint/src/fix/mod.rs`
- Registry: `crates/djangofmt_lint/src/registry.rs`
- Checker: `crates/djangofmt_lint/src/checker.rs`
- Shared helpers: `crates/djangofmt_lint/src/rules/helpers.rs`
- Test runner: `crates/djangofmt_lint/tests/check/main.rs`
- Test fixtures with fix snapshot: `crates/djangofmt_lint/tests/check/untrimmed_blocktranslate/`

---
name: add-lint-rule
description: Add or port a djangofmt_lint rule end to end, from scoping to the single landing commit. Use when adding or porting a lint rule.
---

# Add Lint Rule

## Before starting

Pin down the rule's **full scope** — *every* distinct check it must perform — before writing code. A rule that maps to a spec but implements only its obvious half ships broken.

- **Rule maps to a spec or accessibility technique?** Treat that source's **test procedure** as the checklist and implement every step of it. WCAG technique H63, for instance, is two checks — every `<th>` has a `scope` *and* every `scope` value is one of `row`/`col`/`rowgroup`/`colgroup`; a rule that only checks presence is half a rule. Cross-check how established linters (axe-core, Nu HTML Checker, eslint-plugin-jsx-a11y) cover the same concern to surface dimensions you'd otherwise miss — and keep concerns *they* split into separate rules separate (scope-on-a-non-`<th>` is its own rule, not part of this one).
- **Rule ported from another linter?** Its tests and issue tracker hold the edge cases that bit real users.

Both porting research and grounding fixtures in real code have exact mechanics — see **[research.md](research.md)**.

## Step 1: Create the rule file

Create `crates/djangofmt_lint/src/rules/{category}/{rule_name}.rs`.

Categories map to `RuleCategory`: `correctness`, `suspicious`, `style`, `complexity`, `accessibility`, `pedantic`.

Every category but `pedantic` runs by default. Put a rule in `pedantic` when it is noisy on real templates and has no safe fix, unless it catches something important. Measure noise with `just ecosystem-check-lint-dev` before deciding.

The file must contain:

### 1a. The violation struct with doc comment

The rule's documentation lives in exactly one place: a single `///` doc comment attached directly to the struct. `#[derive(ViolationMetadata)]` (step 1b) reads it from there to populate `docs/rules/{name}.md`, so prose written anywhere else in the file reaches no reader.

Write it for the template author reading the rendered docs: **what** the rule flags and why, NEVER **how** the implementation detects it. Detection mechanics — path matching, gates, skip conditions — live in the code; in the docs they are noise that goes stale.

`RedundantTypeAttr` is the canonical model — mirror its structure exactly, including where the blank `///` lines fall:

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
/// (Only when the fix is unsafe.) State what makes it unsafe — e.g. "marked
/// as unsafe: rewriting the scheme changes which endpoint the browser
/// requests."
///
/// ## Options
/// - `lint.my-rule.some-option`
///
/// ## References
/// - [Spec/docs link](https://example.com/relevant-spec)
#[derive(Debug, PartialEq, Eq)]
pub struct MyRule {
    // Fields used in message() and help()
}
````

Formatting rules:

- **Line width**: fill each line to column 100 (the workspace `rustfmt` width), counting the `///` prefix; wrapping earlier wastes vertical space and churns diffs when neighbouring text is edited.
- `## What it does` — one sentence, starts with "Checks for".
- `## Why is this bad?` — declarative and plain, in the third person ("`eval()` is insecure as it enables arbitrary code execution").
- `## Example` — HTML or Jinja markup; `Use instead:` is plain text between the two fences.
- `## Fix safety` — include only when the fix is unsafe or conditionally unsafe. One short paragraph on what makes it unsafe. A safe fix carries no section; the `Fix` column of the rules table already reports that the rule is fixable.
- `## Options` — include only when the rule reads settings from `pyproject.toml`. Bullet list of dotted option paths in backticks, nothing else: the generator turns each into a link to `docs/settings.md` and fails on unknown options. Document the option itself (doc comment, default, type, example) on its field in `pyproject.rs`, never in the rule.
- `## References` — include when there is a relevant spec, framework doc, or upstream issue to link. Bullet list, one link per line, primary sources only (WHATWG/W3C specs, MDN, framework documentation, CWE/OWASP). A cross-reference to another linter's rule belongs in the PR description.

### 1b. Derive `ViolationMetadata` and declare the lifecycle

Add `ViolationMetadata` to the struct's `#[derive(...)]` list, then add a `#[violation_metadata(...)]` attribute declaring the rule's lifecycle status. The derive captures the doc comment above and records the source file/line; the attribute records which version introduced the rule and whether it ships on by default. Both feed the `djangofmt_dev` generator that builds `docs/rules/{name}.md` and `docs/rules.md`.

```rust
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct MyRule {
    // Fields used in message() and help()
}
```

**The version is always the `NEXT_DJANGOFMT_VERSION` placeholder.** `release.sh` rewrites every occurrence under `crates/djangofmt_lint/src/rules` to the real version at release time, so a new rule cannot know which release it ships in. Existing rules show literal versions only because a past release already stamped them.

Pick the lifecycle keyword for the rule:

- `stable_since` — enabled by default. Use for clear, low-false-positive rules.
- `preview_since` — only runs in preview mode. Use while a new or heuristic rule is validated against real templates, so it is not on by default until proven.

(`deprecated_since` / `removed_since` also exist, for retiring a rule.)

Import the derive and the message-format attribute alongside `Violation`:

```rust
use std::borrow::Cow;

use crate::violation::{Violation, ViolationMetadata, derive_message_formats};
```

### 1c. Implement `Violation`

```rust
impl Violation for MyRule {
    const RULE: Rule = Rule::MyRule;
    const CATEGORY: RuleCategory = RuleCategory::Style; // pick the right one
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always; // omit if no fix

    #[derive_message_formats]
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

`#[derive_message_formats]` is required on `message()`: it supplies the trait's `message_formats()`, which the docs generator reads for the Message column of the rules table.

`FIX_AVAILABILITY` defaults to `FixAvailability::None`; omit it for fixless rules. Use `FixAvailability::Always` when every diagnostic carries a fix, `FixAvailability::Sometimes` when the fix is conditional. A fix that preserves runtime semantics is a `Fix::safe_edit`; one that can change what the page does is a `Fix::unsafe_edit`, and it is the unsafe case that earns the `## Fix safety` section in the docstring.

Diagnostic wording: `message()` names the problem in a short phrase that echoes the rule name; `help()` is one imperative clause naming the action. Keep both under a dozen words and end on the last word rather than a period — the reasoning belongs in `## Why is this bad?`, not in the diagnostic.

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

The signature follows the AST node the rule inspects, and picks the `visit_*` hook that dispatches it in Step 4: attribute rules take `(checker, attr, element)` or `(checker, attr)` from `visit_native_attribute`, element rules `(checker, element)` from `visit_element`, Jinja block rules `(checker, block)` from `visit_jinja_block`. Most rules are attribute rules:

```rust
pub fn check(checker: &Checker<'_>, attr: &NativeAttribute<'_>, element: &Element<'_>) {
    // One `let…else` both destructures the attribute and requires a value;
    // the paired offset is redundant with `source_span(value_str)`.
    let NativeAttribute { name, value: Some((value_str, _)), quote } = attr else { return; };
    if !name.eq_ignore_ascii_case("type") || contains_interpolation(value_str) {
        return;
    }
    // ... decide whether `value_str` is a violation on this `element`
    let mut guard = checker.report_diagnostic(&violation, checker.source_span(value_str));
    // Only for rules with a fix; attach it before the guard drops:
    guard.set_fix(delete_attr_fix(checker.context(), name, value_str, quote.is_some()));
}
```

Key points:

- The `Checker` is passed as `&Checker<'_>`; diagnostics are buffered through interior mutability (`RefCell`), so no `&mut` is needed.
- `checker.report_diagnostic(&violation, span)` returns a `DiagnosticGuard`. On `Drop` the guard pushes the diagnostic into the context's buffer. Hold the guard in a `let mut guard = ...` binding only if you need to attach a fix or override fields; otherwise let the temporary drop immediately.
- If the rule is **not** gated upfront in `checker.rs` (Step 4), call `checker.report_diagnostic_if_enabled(...)` instead — it returns `Option<DiagnosticGuard>` and short-circuits when disabled.
- For fixes: build an `Edit` (`Edit::deletion`, `Edit::insertion`, `Edit::replacement`) and wrap it with `Fix::safe_edit(...)` or `Fix::unsafe_edit(...)`, then call `guard.set_fix(fix)`. Deleting a whole attribute is `delete_attr_fix` from `fix/edits.rs`, which widens the deletion over the surrounding whitespace.
- **A slice locates itself**: build spans with `checker.source_span(slice)`. `checker.source_offset(slice)` / `checker.source_end(slice)` are its bounds, for range arithmetic; the free `span(start, len)` is for offsets no slice provides.
- **Report the narrowest span that names the problem** — the offending value or attribute, not the whole element. Each failure mode can point at its own slice: `source_span(value)` for a bad value, `source_span(attr.name)` for a bad attribute, `source_span(element.tag_name)` when the element itself is at fault.
- **Match HTML case-insensitively** — tag names, attribute names, and enumerated values alike: `name.eq_ignore_ascii_case("scope")`, `value.eq_ignore_ascii_case("col")`.

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

## Step 4: Wire it in the checker

Add the rule check call in the `visit_*` method matching the signature chosen in 1d, in `crates/djangofmt_lint/src/checker.rs`, gated by `is_rule_enabled`:

```rust
fn visit_native_attribute(&self, attr: &NativeAttribute<'a>, element: &Element<'a>) {
    if self.is_rule_enabled(Rule::MyRule) {
        rules::style::my_rule::check(self, attr, element);
    }
    // ...
}
```

If the rule's `check` uses `report_diagnostic_if_enabled` internally instead of being gated here, you can skip the `is_rule_enabled` wrapper — but gating upfront is cheaper when the rule does any non-trivial work before reporting.

**Several rules keyed off the same tag?** See **[checker-gating.md](checker-gating.md)** for classify-once dispatch.

## Step 5: Create test fixtures

Create directory `crates/djangofmt_lint/tests/check/{rule_name}/` with two files.
The directory name must be the rule code with underscores — the test runner derives
the rule from it and runs **only that rule**, so fixtures never trip other rules'
diagnostics (and a coverage test fails if the directory is missing or misnamed).
A `.jinja` extension instead of `.html` runs the fixture under the Jinja profile,
the only one that allows whitespace-control markers.

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

Adapt every issue-derived and Sourcegraph case from **[research.md](research.md)** into HTML/Jinja, keeping its inline citation comment. Every edge case the original tests exists for a reason.

## Step 6: Run tests and accept snapshots

```bash
uv run --only-dev cargo insta test --accept -p djangofmt_lint
```

Snapshots auto-generated by the test runner in `tests/check/main.rs`:

- `{rule_name}.invalid.snap` — rendered diagnostics for the invalid fixture.
- `{rule_name}.invalid.fixed.snap` — post-fix source, written when the rule attaches a safe `Fix`.
- `{rule_name}.invalid.unsafe-fixed.snap` — post-fix source under `--unsafe-fixes`, written when the rule attaches an unsafe `Fix`.

Read every diagnostic in every snapshot before moving on: each span underlines the offending value rather than the whole element, each message and help follows the wording rules in 1c, and the fixed source is markup you would have written by hand — whitespace intact, surrounding tags untouched.

## Step 7: Smoke-test against real templates

Run the new rule over a large corpus of real-world templates. `just ecosystem-check-lint-dev` is the ready-made corpus: it diffs the debug build against the installed `djangofmt` over the ecosystem repos with every rule selected, preview included, so each reported diagnostic is the new rule's. Any large template project works too:

```bash
cargo run -p djangofmt -- check --select {rule-slug} path/to/templates
```

Add `--preview` if the rule is `preview_since`. Account for every diagnostic the run produces: each one is either a true positive, or a legitimate pattern that moves into `{rule_name}.valid.html` with the rule taught to skip it. The step is done when every remaining diagnostic is a true positive.

## Step 8: Pre-merge check and proofread the rendered docs

```bash
just pre-mr-check
```

Get it green. Its docs build regenerates `docs/rules.md` and `docs/rules/{rule_name}.md` from the doc comment; the output is gitignored (`docs/.gitignore`), so commit nothing from it. Then read `docs/rules/{rule_name}.md`: every section from 1a present in order, both example fences rendered, every reference link resolving, and the prose reading as the template author will see it.

## Step 9: Commit as a single commit

A new rule lands as **one** commit on a branch. Squash review follow-ups into it before pushing.

**Title**: ``feat(lint): Add `{rule-slug}` lint rule`` — the `feat(lint):` conventional-commit prefix, then backticks around the kebab-case slug (e.g. ``feat(lint): Add `javascript-url` lint rule``).

**Body**: the rule's docstring verbatim, `///` prefix stripped — section headings (`## What it does` through `## References`), prose, and code fences exactly as written in the source. This duplicates the doc comment into the commit message on purpose: the body is what reviewers and `git log` readers see, and it is the single best place to capture the rationale at the time of landing.

## Reference files

- Reference attribute rule with safe fix (doc layout, `DiagnosticGuard`, `delete_attr_fix`): `crates/djangofmt_lint/src/rules/style/redundant_type_attr.rs`
- Reference rule with unsafe fix (`## Fix safety` section, `Fix::unsafe_edit`): `crates/djangofmt_lint/src/rules/suspicious/use_https.rs`
- Reference rule without fix: `crates/djangofmt_lint/src/rules/correctness/invalid_attr_value.rs`
- Multi-variant (enum) violation with `match` in `message()`/`help()`: `crates/djangofmt_lint/src/rules/accessibility/missing_title.rs`
- Full-technique rule (WCAG H63) — multi-variant, per-variant spans, case-insensitive value validation: `crates/djangofmt_lint/src/rules/pedantic/table_header_missing_scope.rs`
- Jinja-block-shaped rule with fix: `crates/djangofmt_lint/src/rules/correctness/untrimmed_blocktranslate.rs`
- Violation trait: `crates/djangofmt_lint/src/violation.rs`
- `LintContext` / `DiagnosticGuard`: `crates/djangofmt_lint/src/lint_context.rs`
- Fix data model (`Edit`, `Fix`, `Applicability`, `FixAvailability`): `crates/djangofmt_lint/src/fix/mod.rs`
- Fix helpers (`delete_attr_fix`): `crates/djangofmt_lint/src/fix/edits.rs`
- Registry: `crates/djangofmt_lint/src/registry.rs`
- Checker: `crates/djangofmt_lint/src/checker.rs`
- Shared helpers: `crates/djangofmt_lint/src/rules/helpers.rs`
- Test runner: `crates/djangofmt_lint/tests/check/main.rs`
- Test fixtures with fix snapshot: `crates/djangofmt_lint/tests/check/untrimmed_blocktranslate/`

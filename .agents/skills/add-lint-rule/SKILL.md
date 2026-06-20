---
name: add-lint-rule
description: Use this skill when adding a new lint rule to djangofmt_lint. Guides through creating the violation struct, check function, registry entry, checker wiring, and test fixtures with valid/invalid convention.
---

# Add Lint Rule

## Overview

This skill walks through adding a new lint rule to the `djangofmt_lint` crate. Every rule follows the same structure — a violation struct, a check function, registry wiring, and test fixtures.

## Before starting

If the rule is being ported from another linter (djLint, etc.), do **both** of these before writing fixtures:

1. **Read the original rule's test suite and documentation** — use those cases to inform test coverage (don't just port the happy path).
2. **Search the upstream issue tracker** for the rule code (e.g. `H019`) and the rule slug (e.g. `javascript-url`), in both open and closed state. Read every issue and PR that mentions the rule. Pay attention to:
   - **Closed feature requests that were rejected** — they encode "we deliberately don't flag X" decisions. Each one becomes a `valid.html` case with an inline `<!-- Regression: <linter> #<num> — ... -->` comment citing the issue, so a future reviewer can't quietly remove it.
   - **Closed false-positive bug reports** — same treatment; lock the case into `valid.html` with an issue cite.
   - **Original feature request that birthed the rule** — the minimal reproducer there is often the canonical true-positive; mirror it in `invalid.html` with an issue cite.

   For GitHub repos: `gh search issues --repo <owner>/<repo> "<RULE_CODE> OR <rule-slug>" --limit 30 --json number,state,title`. Spend a couple of minutes; this is the cheapest place to find out which edge cases bit real users.

## Step 1: Create the rule file

Create `crates/djangofmt_lint/src/rules/{category}/{rule_name}.rs`.

Categories map to `RuleCategory`: `correctness`, `suspicious`, `style`, `complexity`, `accessibility`, `nursery`.

The file must contain:

### 1a. The violation struct with doc comment

The doc comment on the struct **is** the rule's documentation. It MUST be a `///` doc comment attached directly to the struct, not a `//!` module-level comment at the top of the file — Rust convention is to attach documentation to the item it describes, and the `#[derive(ViolationMetadata)]` macro (see step 1b) reads it from the struct to populate `docs/rules/{name}.md`.

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
- `## Why is this bad?` — content immediately after the heading. Add follow-up paragraphs (separated by blank `///`) for exclusions or non-obvious behaviour. Match Ruff's voice: declarative, plain prose ("`eval()` is insecure as it enables arbitrary code execution"). Avoid editorial flair like "classic X sink" or "brittle across browsers", filler adjectives, and second-person ("you"). For models: skim a few rules under [astral-sh/ruff `crates/ruff_linter/src/rules/`](https://github.com/astral-sh/ruff/tree/main/crates/ruff_linter/src/rules) (e.g. `flake8_bandit/rules/suspicious_function_call.rs`) before drafting.
- `## Example` — code fence on the line **immediately after** the heading. No blank `///` between heading and `` ```html ``. After the closing fence, blank line, then plain text `Use instead:` (NOT a sub-heading, no `##`), then the corrected code fence immediately on the next line. Use HTML/Jinja, not Python.
- `## Fix safety` — include only when the rule registers a fix. One short paragraph documenting the safety classification and why.
- `## References` — include when there is a relevant spec, framework doc, or upstream issue to link. Bullet list, one link per line. Link primary sources only (WHATWG/W3C specs, MDN, framework documentation, CWE/OWASP). Do **not** link other linters' rule pages (djLint, Ruff, ESLint, etc.) — even when the rule was ported from one, the cross-reference belongs in the PR description, not in the user-facing docs.

Do NOT add a separate `//!` file-level header or a second `///` "Violation for X" doc block — the struct doc comment is the single source of truth.

### 1b. Derive `ViolationMetadata`

Add `ViolationMetadata` to the struct's `#[derive(...)]` list. The derive captures the doc comment above and records the source file/line, which the `djangofmt_dev` generator uses to build `docs/rules/{name}.md` and `docs/rules.md`.

```rust
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
pub struct MyRule {
    // Fields used in message() and help()
}
```

Import the trait alongside `Violation`:

```rust
use crate::violation::{Violation, ViolationMetadata};
```

### 1c. Implement `Violation`

```rust
impl Violation for MyRule {
    const RULE: Rule = Rule::MyRule;
    const CATEGORY: RuleCategory = RuleCategory::Style; // pick the right one
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always; // omit if no fix

    fn message(&self) -> String {
        // Concise, includes relevant values from fields
    }

    fn help(&self) -> Option<String> {
        // Actionable fix suggestion, or None
    }

    // Only for rules that produce a fix:
    fn fix_title(&self) -> Option<String> {
        Some("Short imperative summary".to_string())
    }
}
```

`FIX_AVAILABILITY` defaults to `FixAvailability::None`; omit it for fixless rules. Use `FixAvailability::Always` when every diagnostic carries a fix, `FixAvailability::Sometimes` when the fix is conditional.

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

The function signature depends on what AST node the rule inspects. Element-level rules take `&Element<'_>`; Jinja block rules take `&JinjaBlock<'_, Node<'_>>`.

**Cross-node rules** — those that decide from many nodes at once (a duplicate name across the whole template, "`{% extends %}` must come first") — can't judge from a single node. Don't re-walk the tree; that drifts from the canonical traversal. Mirror ruff's **deferred analysis**: accumulate state into a `Checker` field during the single existing pass, then run a **finalize** `check(checker)` after traversal completes (call it in `visit_root`, after the `root.children` loop). To hold borrowed `&'a str` slices from the AST in that field, thread the source lifetime `'a` through the `visit_*` signatures (`&Root<'a>`, `&Element<'a>`, …) and `check_ast` — the AST already borrows from the source, and covariance keeps callers (`fix_ast`, `lint_fix`) compiling unchanged. See `duplicate_block_name.rs`.

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

## Step 5: Create test fixtures

Create directory `crates/djangofmt_lint/tests/check/{rule_name}/` with two files:

### `{rule_name}.invalid.html`

Contains **only** cases that should produce diagnostics. Every line/block that looks like it might be valid should be in the valid file instead. Group cases with HTML comments (`<!-- Case-insensitive tag name -->`) — the snapshot becomes much easier to review.

### `{rule_name}.valid.html`

Contains cases that must produce **zero** diagnostics. This file is critical — it catches false positives.

The runner lints every fixture with **all rules enabled** (`Settings::all()`), so a `valid.html` case must be clean against *every* rule, not just yours. A guard case that strays into another rule's territory has to satisfy that rule too — a `{% blocktranslate %}` case needs `trimmed`, an `<html>` case needs `lang` — or that rule's diagnostic breaks the zero-diagnostics assertion.

Include:

- The obvious correct usage
- Edge cases: interpolated values (`{{ var }}`, `{% if %}`), missing attributes, empty values
- Elements that look similar but shouldn't trigger (e.g., `<div method="put">` for a form-only rule)
- Boundary cases from the original linter's test suite if porting
- **Every false-positive case and every rejected "extend the rule to X" feature request found in the upstream issue tracker** (see "Before starting"). Each gets an inline `<!-- Regression: <linter> #<num> — <one-line rationale> -->` comment so the case can't be silently removed later.

### If porting from another linter

Before writing fixtures, read the original linter's test cases AND search its issue tracker (see "Before starting"). Adapt the cases to HTML/Jinja syntax. Don't drop edge cases — if the original tests something, there's usually a reason. Anchor each issue-derived case with an inline regression comment citing the issue number.

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

## Step 7: Regenerate the rule documentation

```bash
just docs-generate
```

This refreshes `docs/rules.md` and writes `docs/rules/{rule_name}.md` from the violation struct's doc comment. CI runs `--mode check` against these files, so the regenerated pages must be committed.

## Step 8: Commit as a single commit

A new rule lands as **one** commit on a branch, not a stack of "add", "fix tests", "simplify", "doc tweak" commits. Squash any review follow-ups into that single commit before pushing.

**Title**: ``Add `{rule-slug}` lint rule`` — backticks around the kebab-case slug (e.g. ``Add `javascript-url` lint rule``). No conventional-commit prefix.

**Body**: the rule's docstring content, with the `///` prefix stripped, preserving the section structure (`## What it does`, `## Why is this bad?`, `## Example`, `## Fix safety` if applicable, `## References`). This duplicates the doc comment into the commit message on purpose: the body is what reviewers and `git log` readers see, and it is the single best place to capture the rationale at the time of landing.

Example:

```text
Add `javascript-url` lint rule

## What it does
Checks for `javascript:` URLs in HTML elements.

## Why is this bad?
`javascript:` URLs execute arbitrary code when the element is activated.
Any data interpolated into the URL becomes executable, which can allow cross-site scripting
(XSS) attacks. The pattern also bypasses Content Security Policy `script-src` directives.
Use a real URL and attach behavior with an event handler instead.

## Example
` ` `html
<a href="javascript:alert('Hello, world!')">Click me</a>
` ` `

Use instead:
` ` `html
<button id="btn">Click me</button>
<script>
  document.getElementById("btn").addEventListener("click", () => {
    alert("Hello, world!");
  });
</script>
` ` `

## References
- [MDN: `javascript:` URLs](https://developer.mozilla.org/en-US/docs/Web/URI/Reference/Schemes/javascript)
```

(In the actual commit message, write triple backticks `` ``` `` for the fences; spaces are only used above to keep this example renderable inside the skill file.)

## Checklist

- [ ] Doc comment attached to the violation struct (not `//!`) following the `RedundantTypeAttr` layout exactly: `## What it does`, `## Why is this bad?`, `## Example` (code fence on the next line, `Use instead:` as plain text, no `##` sub-heading), `## Fix safety` if the rule fixes, `## References` when relevant
- [ ] `#[derive(ViolationMetadata)]` added to the struct so the doc comment is captured for `docs/rules/{name}.md`
- [ ] Violation struct with `message()` and `help()` (and `fix_title()` + `FIX_AVAILABILITY` if it fixes)
- [ ] Check function takes `&Checker<'_>` (not `&mut`), with early returns and interpolation skipping
- [ ] Fix attached via `guard.set_fix(Fix::safe_edit(...))` before the guard drops
- [ ] Module exported in `rules/{category}/mod.rs`
- [ ] Registered in `define_rules!` in `registry.rs`
- [ ] Wired in `checker.rs` visitor under `is_rule_enabled`
- [ ] `{rule_name}.invalid.html` with diagnostic-producing cases
- [ ] `{rule_name}.valid.html` with false-positive-catching cases, including a regression case (with issue cite) for every rejected "extend to X" request and every false-positive bug found in the upstream issue tracker
- [ ] If porting: upstream issue tracker searched and findings reflected in fixtures
- [ ] Snapshot(s) reviewed and accepted — `.invalid.snap` and `.invalid.fixed.snap` for rules with fixes
- [ ] `just docs-generate` run and the regenerated `docs/rules/{rule_name}.md` + updated `docs/rules.md` committed
- [ ] `cargo test` passes
- [ ] smoke test running on `~/greenday` does not reveal any issues
- [ ] Single commit titled ``Add `{rule-slug}` lint rule`` with the rule's docstring as the body

## Reference files

- Reference rule with safe fix (doc layout, `DiagnosticGuard`, `Fix::safe_edit`): `crates/djangofmt_lint/src/rules/style/redundant_type_attr.rs`
- Reference rule without fix: `crates/djangofmt_lint/src/rules/correctness/invalid_attr_value.rs`
- Jinja-block-shaped rule with fix: `crates/djangofmt_lint/src/rules/correctness/untrimmed_blocktranslate.rs`
- Cross-node rule (accumulate during the pass + finalize after; threads the source lifetime): `crates/djangofmt_lint/src/rules/correctness/duplicate_block_name.rs`
- Violation trait: `crates/djangofmt_lint/src/violation.rs`
- `LintContext` / `DiagnosticGuard`: `crates/djangofmt_lint/src/lint_context.rs`
- Fix data model (`Edit`, `Fix`, `Applicability`, `FixAvailability`): `crates/djangofmt_lint/src/fix/mod.rs`
- Registry: `crates/djangofmt_lint/src/registry.rs`
- Checker: `crates/djangofmt_lint/src/checker.rs`
- Shared helpers: `crates/djangofmt_lint/src/rules/helpers.rs`
- Test runner: `crates/djangofmt_lint/tests/check/main.rs`
- Test fixtures with fix snapshot: `crates/djangofmt_lint/tests/check/untrimmed_blocktranslate/`

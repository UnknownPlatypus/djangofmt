# Plan: `unused-ignore-code` lint rule (ruff RUF100 `unused-noqa` equivalent)

Part 5 of #436, stacked on #442 (`ignore-comment-lints`). Sibling of `invalid-ignore-comment`,
`invalid-ignore-code` (#442) and `redirected-ignore` (#443).

## Context

- Node-level `ignore[...]` suppression is applied in `suppression::drop_ignored_diagnostics` by
  filtering the diagnostic buffer after every rule ran. Nothing records which codes matched.
- File-level `file-ignore[rule]` works differently: `LintContext::new` removes the rule from the
  run's `RuleSet` via `file_ignored_rules`, so the rule never executes.
- The directive meta-rules (`visit_ignore_comments`) run after filtering, so their diagnostics can
  only be silenced by that file-level narrowing.
- #443 (`redirected-ignore`) was cut from an older version of `ignore-comment-lints`, before the
  `IgnoreComment` / `ReservedCode` / `delete_codes_or_comment` rework. It must be rebased; only its
  checker hook (`NodeKind::Comment`) and registry line survive.
- Every directive in `~/greenday` (1511 files) is the legacy bare `{# djangofmt:ignore #}`, which
  the linter never parses as an `IgnoreComment`. The smoke test there yields nothing by construction.

Ruff reference: `crates/ruff_linter/src/checkers/noqa.rs` (`check_noqa`) and
`rules/ruff/rules/unused_noqa.rs`.

## Design decisions

- **Name / category:** `unused-ignore-code`, `Suspicious`, `stable_since = "NEXT_DJANGOFMT_VERSION"`,
  file `crates/djangofmt_lint/src/rules/suspicious/unused_ignore_code.rs`.
- **Bookkeeping via `&mut`, not `Cell<bool>`.** `retain_diagnostics` takes an `FnMut` closure and
  the comment list is a local `Vec` in `check_ast`. Add `matched: RuleSet` to `IgnoreComment`,
  make `drop_ignored_diagnostics` take `&mut [IgnoreComment]`, insert the dropped diagnostic's
  `Rule` into the matching comment's set. A code is used iff its `Rule` is in `matched`.
- **Cover `file-ignore[...]` too (ruff parity).** Stop narrowing the `RuleSet` in
  `LintContext::new`; give the leading `FileIgnore` comment a guarded range spanning the whole
  file so one filtering path serves both directives and records matches for both.
  `file_ignored_rules` loses its only caller and goes away. Perf cost (running per-file-disabled
  rules) is negligible.
- **Reorder `check_ast`:** collect comments -> run `invalid-ignore-comment` and
  `invalid-ignore-code` -> filter + record matches -> run `unused-ignore-code` last. Keeps
  `file-ignore[invalid-ignore-code]` working once narrowing is gone; matches ruff's ordering.
- **Self-suppression.** A comment listing `unused-ignore-code` is never reported and that code
  counts as used. A leading `file-ignore[unused-ignore-code]` turns the rule off for the file.
- **Code classification, per comment:**
  - unknown code (no `Rule`, no `ReservedCode`): skipped, owned by `invalid-ignore-code`
  - `format`: always used (the linter cannot see the formatter)
  - `invalid-syntax` in a leading `file-ignore`: unused when the file parsed
  - rule code: `unmatched` (enabled, never fired), `disabled` (rule off in settings, ruff's
    "non-enabled"), `duplicated` (repeat of an earlier code in the same list)
- **One diagnostic per comment.** Message in the sibling's style:
  `` Unused rule code in suppression: `a`, `b` `` with `non-enabled:` / `duplicated:` segments when
  relevant. Fix title `Remove unused rule code` or `Remove suppression comment`. Span from
  `delete_codes_or_comment`: the code when one goes, the comment otherwise.
- **Fix safety follows the siblings:** dropping codes from a list is safe; deleting the whole
  comment is unsafe (the free-text reason goes with it).
- **Duplicates need an edits change.** `delete_codes_or_comment` removes every occurrence of a
  code, which would delete both copies of a duplicated valid code. Change `remove: &[&str]` to a
  set of indices so the caller drops only the repeat. Both existing callers adapt in one line.
- **Fix loop:** no change. Suppressed diagnostics are never fixed, so their codes stay used across
  iterations. When two rules edit the same comment, `apply_fixes` rejects the overlap and the
  second edit lands next iteration.

## Implementation steps

1. Rebase #443 onto the current `ignore-comment-lints` head (checker hook + registry line only).
2. Commit 1 (refactor only): `matched: RuleSet` on `IgnoreComment`, whole-file guarded range for
   a leading `FileIgnore`, `&mut` filtering that records matches, `LintContext::new` without
   narrowing, `check_ast` reordered. Existing `suppression.rs` tests stay green; add one asserting
   `file-ignore[invalid-ignore-code]` still silences that rule.
3. Commit 2: `delete_codes_or_comment` takes indices. Adapt both callers and the doc examples.
4. Commit 3 (the rule): violation struct + doc comment per `add-lint-rule`, `check(comments,
   checker)`, self-ignore checks, registry entry, `visit_ignore_comments` call, `pub mod` export,
   third bullet in the "Suppressing diagnostics" section of `docs/rules.md`. Tick RUF100 in `t.md`.
5. Squash into ``feat(lint): Add `unused-ignore-code` lint rule`` with the docstring as body.

## Tests and verification

- **Fixture dir** `crates/djangofmt_lint/tests/check/unused_ignore_code/`. The harness runs only the
  rule under test, so any real rule code reads as non-enabled there. Invalid fixture: non-enabled
  wording, duplicates, targetless comment at end of file, whole-comment deletion with a reason,
  a `file-ignore` list. Valid fixture: `ignore[format]`, self-ignore, an unknown code left alone.
- **Unit tests in `suppression.rs`** with `Settings::all()`, next to the existing end-to-end ones:
  a used code stays; an unmatched code among used ones is reported alone; a stale `file-ignore` is
  reported; `invalid-syntax` on a parsing file is reported.
- **Smoke test:** scratch copy of a few greenday templates with hand-written stale and live
  `ignore[...]` comments, `check --select unused-ignore-code` with and without `--fix`.
- `just pre-mr-check`, `just docs-generate` to proofread the rendered rule page.

## Open decisions

1. Disabled rules count as unused (ruff parity, recommended) or as used (gentler with `--select`
   on the CLI, which otherwise strips every comment under `--fix`)?
2. `file-ignore` coverage in this PR (recommended) or as a follow-up to keep the first PR to
   node-level comments?

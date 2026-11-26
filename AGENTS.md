## Conventions

- Use comments sparingly — only when they explain hidden behavior — and keep them to 1-2 lines. Rule doc comments (`## What it does` …) are the generated docs and exempt.
- Core lint and format infrastructure mirrors ruff's design upfront, even before a rule needs it. Peripheral features (editorconfig, ecosystem-check tooling) stay minimal and match a reference tool's scope, not the full spec.
- Every new CLI flag must also be readable from `pyproject.toml`: add it to `PyprojectSettings`/`LintSettings` in `pyproject.rs` and resolve it with CLI args taking precedence.
- Keep tests lean: one case per behaviour, and a case another case already exercises is deleted.
- Adding or porting a lint rule: follow `.agents/skills/add-lint-rule/SKILL.md` end to end.
- Babysitting a PR to green (conflicts, failing CI jobs, review threads, CodSpeed, ecosystem check): follow `.agents/skills/babysit-pr/SKILL.md`.
- Branches are squash-merged under the PR title, which becomes the changelog entry, so the PR title carries a conventional-commit `type(scope):` prefix, e.g. `feat(lint):`, `fix(format):`, `chore(deps):`. Commits get plain descriptive titles, except on a single-commit branch: GitHub reuses that commit's title as the PR title, so it carries the prefix.
- PR descriptions and every thread comment or reply: write like a busy maintainer, in short plain sentences. A PR description is at most two sentences on what changes and why, plus a `Closes #N`/`References #N` line when there is one. When the PR changes formatter output or lint diagnostics, show it with one small block: a `diff` of a template before/after formatting, or a snippet with the diagnostic it raises. Use one bullet per change only when the PR makes several unrelated changes, and add no headings, follow-up steps, or recap of the diff. A review reply answers the question or names the fix in one short sentence.
- Pass PR and comment bodies to `gh` with `--body-file -` and a `<<'EOF'` heredoc, with backticks left unescaped.
- Keep commit descriptions minimal or empty — write a body only when explicitly asked, or when a skill documents otherwise (e.g. `add-lint-rule`).

## Commands

```bash
just                                  # List every recipe (coverage, benchmarks, ecosystem checks, fuzzing, docs)
just pre-mr-check                     # Full pre-merge check: pre-commit, clippy, all tests, docs build
uv run --only-dev cargo insta review  # Accept/reject snapshot mismatches
```

## Architecture

`djangofmt_syntax` sits at the bottom and depends on neither the formatter nor the linter. `djangofmt_formatter` and `djangofmt_lint` build on it independently. The `djangofmt` CLI, wasm and benchmark crates combine them; the CLI holds no formatting or lint logic of its own.

Formatter fixes get a fixture in `crates/djangofmt/tests/fmt/`: the harness formats every fixture twice, so it also catches unstable output.

### `markup_fmt`

The HTML/Jinja2 parser and formatter that provides the AST, pinned to a git rev of the `UnknownPlatypus/markup_fmt` fork in the root `Cargo.toml`. Fixes to it go in a local checkout of the fork as two branches: one on top of `django/baseline`, which djangofmt then pins, and a cherry-pick of the same commits on top of `g-plane/main` for the upstream PR.

To test an unpushed fork commit, point the workspace dependency at `{ path = "../markup_fmt/markup_fmt" }`: `[patch]` cannot resolve an unpushed rev. Restore the `{ git, rev }` line and `Cargo.lock` before committing.

## Conventions

- Use comments sparingly — only when they explain hidden behavior — and keep them to 1-2 lines.
- Keep tests lean: cover the highest-value cases and refactor them to a high standard.
- Commit titles use a conventional-commit `type(scope):` prefix, e.g. `feat(lint):`, `fix(format):`, `chore(deps):`.
- Keep commit descriptions minimal or empty — write a body only when explicitly asked, or when a skill documents otherwise (e.g. `add-lint-rule`).

## Commands

```bash
just pre-mr-check          # Full pre-merge check: pre-commit, clippy, all tests
cargo test --workspace --all-targets --all-features  # Run all tests
cargo test -p djangofmt --test cli <test_name>       # Run a specific CLI integration test
cargo test -p djangofmt --test fmt                   # Run all formatting snapshot tests
cargo clippy --all-targets --all-features            # Lint
cargo insta review                                   # Accept/reject snapshot mismatches after a formatter change
just coverage                                        # HTML coverage report
just bench-rs                                        # Rust micro-benchmarks
```

## Architecture

This is a Rust workspace with six crates in `crates/`:

**`djangofmt`** — the main CLI binary. Discovers files via `resolver.rs`, reads config from `pyproject.toml` (`[tool.djangofmt]`) merged with CLI args (`args.rs`), then processes files in parallel using rayon. The `format` command reformats in-place; the `check` command reports violations. Error reporting uses `miette`. Exit codes: 0 = success, 1 = formatting/lint errors, 2 = I/O or parse errors.

**`djangofmt_lint`** — the linting library. `checker.rs` implements a visitor over the `markup_fmt` AST and runs rules from `rules/`. Each rule implements the `Violation` trait. The `Checker` struct collects violations with source offsets. Adding a rule is an end-to-end process covered by the `add-lint-rule` skill (`.agents/skills/add-lint-rule/SKILL.md`).

**`djangofmt_wasm`** — WebAssembly bindings for the browser playground, built with `wasm-pack`.

**`djangofmt_benchmark`** — divan-based micro-benchmarks embedding real-world templates from Django/Wagtail/Zulip.

**`djangofmt_macros`** — proc macros consumed by `djangofmt_lint`: the `ViolationMetadata` derive and `derive_message_formats`.

**`djangofmt_dev`** — dev-only CLI that generates the rule docs (`docs/rules.md`, `docs/rules/`) from violation doc comments and syncs README/CONTRIBUTING into `docs/`; run via `just docs-generate`. Its output is gitignored, never committed.

### Key external dependencies

- **`markup_fmt`** — the HTML/Jinja2 parser and formatter that provides the AST. An upstream dependency (not vendored), pinned to a git rev of the `UnknownPlatypus/markup_fmt` fork in the root `Cargo.toml`.
- **`malva`** — formats inline CSS in `<style>` tags/attributes.
- **`dprint-plugin-json`** — formats `<script type="application/json">` content.
- **`insta`** / **`insta-cmd`** — snapshot testing for formatter output and CLI behavior respectively.

### Test layout

- `crates/djangofmt/tests/fmt/` — snapshot tests for formatting. Each test is an `.html` input file; expected output is a `.snap` file alongside it.
- `crates/djangofmt/tests/cli.rs` — CLI integration tests using `insta-cmd` and `tempfile`.
- `crates/djangofmt/tests/parse_error/` — parse error reporting tests.
- `crates/djangofmt_lint/tests/` — lint rule tests using a `valid`/`invalid` fixture convention.

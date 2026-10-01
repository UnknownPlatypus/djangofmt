# Plan: Django syntax-aware formatting and lint rules

Issues: [#187](https://github.com/UnknownPlatypus/djangofmt/issues/187) "Integration with djade", [#379](https://github.com/UnknownPlatypus/djangofmt/issues/379) "Normalize whitespace in template tag expressions".
Related: [PR #501](https://github.com/UnknownPlatypus/djangofmt/pull/501) upgrade rules PoC (rebased in step 2), [#79](https://github.com/UnknownPlatypus/djangofmt/issues/79) Jinja expression formatting (out of scope).
Companion page: [Djade Coverage Map](https://claude.ai/artifact/QrPJU1nARDaahYPXeukhRB) holds the corpus counts and the per-feature tables.

Status: **WIP plan, nothing implemented yet.**

Written on 2026-10-02 against djangofmt `1476d3b` (markup_fmt fork `66ff75b`), djade 1.9.0, Django 6.0.

## TL;DR

- One module, `djangofmt_lint::dtl`, with two free functions that reproduce Django's own lexing: `bits()` for tag bodies (`smart_split_re`) and `filter_expression()` for `{{ }}` bodies (`filter_re`). Hand-written scanners, slices borrowed from the input, Django's regexes kept as a randomized test oracle.
- The formatter gains two closure arms, Django profile only, that normalize whitespace inside `{% %}` and `{{ }}`: single spaces between bits, no spaces around filters. The only other formatter changes are two markup_fmt fork items: `{# x #}` edge spacing and one blank line between top-level blocks under `extends`.
- Every other djade behaviour becomes a lint rule with a safe fix: six fixers in a new `upgrade` category, the `endblock`/`endpartialdef` label rule in `style`. All stable, all on by default.
- Not mirrored: the three `load` rules (merge, sort libraries, sort `from` items). One `{% load %}` per line is the preferred style here. djangofmt-then-djade therefore never converges on `load`, documented as a known limitation.
- Dropped from #501: the reverse parity mode and the unset-target-version-to-5.2 default. pretty_jinja stays parked on `pj-rebase`.

## Decisions

| Decision                                 | Choice                                                                                                                                           | Why                                                                                                                                                                                                                            |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Lexer                                    | Path C: hand-written `bits()` and `filter_expression()`                                                                                          | pretty_jinja's Django dialect was byte-identical on the corpus but needs a second fork, seven crates and a public parse API for the linter. A full expression grammar buys the formatter nothing: a Django tag can never wrap. |
| Module home                              | `crates/djangofmt_lint/src/dtl.rs`                                                                                                               | `djangofmt`, `djangofmt_wasm` and `djangofmt_benchmark` already depend on the lint crate.                                                                                                                                      |
| API                                      | Two free functions returning borrowed slices                                                                                                     | `checker.source_span(slice)` gives exact spans for free. No `Tag` struct until a rule needs one.                                                                                                                               |
| Oracle test                              | Unit test with `regex` as a dev-dependency and a seeded xorshift                                                                                 | Runs in CI. `regex` is already in the tree through markup_fmt. No proptest, no fuzz-only check.                                                                                                                                |
| Existing ad-hoc parsers                  | Migrate them in step 1                                                                                                                           | One tokenizer from day one. Snapshots must not change.                                                                                                                                                                         |
| Formatter scope                          | djade's normalization only, Django profile only                                                                                                  | Token changes belong in lint rules users can opt out of. Jinja allows multi-line tags and spaced filters.                                                                                                                      |
| Lint scope                               | djade's set minus `load`                                                                                                                         | No broader rule family planned.                                                                                                                                                                                                |
| Categories                               | `upgrade`: staticfiles, json_script, length_is, ifequal, trans rename, legacy `as`. `style`: labels                                              | `upgrade` means "prefer the newer syntax", like pyupgrade, which also rewrites still-valid forms.                                                                                                                              |
| Default set                              | All new rules stable and on by default                                                                                                           | Default selection already includes every stable non-pedantic category.                                                                                                                                                         |
| Version gates                            | staticfiles 2.1, trans 3.1, ifequal 3.1, json_script 4.1, length_is 4.2; legacy `as` ungated                                                     | Mirrors djade. `key=value` has been valid since Django 1.3.                                                                                                                                                                    |
| `target-version` unset and not inferable | Gated rules stay off                                                                                                                             | Today's behaviour. Drop #501's `OLDEST_SUPPORTED = 5.2` default.                                                                                                                                                               |
| Labels                                   | One `style` rule with a safe fix, not the printer                                                                                                | S instead of M, no fork change, opt-out. Stable when `check --fix` runs after formatting.                                                                                                                                      |
| Checker wiring                           | Every `{% %}` reaches `visit_jinja_tag`, block openers, middles and closers included                                                             | Single-tag rules are written once. Pair rules (labels, ifequal) stay in `visit_jinja_block`.                                                                                                                                   |
| Text tags                                | `tags_in_text` synthesizes `JinjaTag { content, start }` for attribute values, HTML comments and raw bodies, dispatched through the same visitor | 11.5% of trans hits sit there. Lands before the trans rename. Rules opt out with a list like `RAW_SENSITIVE_RULES`.                                                                                                            |
| Parity check                             | Keep only djade-then-djangofmt convergence (`just ecosystem-check-stability djade`)                                                              | Without `load` merging the reverse direction can never pass.                                                                                                                                                                   |
| #501                                     | Rebase onto step 1                                                                                                                               | Keeps step 1 judgeable on its own.                                                                                                                                                                                             |
| Jinja and pretty_jinja                   | Out of scope                                                                                                                                     | `pj-rebase` parked. `integrate-pretty-jinja` and `try-pretty-jinja` deleted.                                                                                                                                                   |

## Tracking

- [ ] **Step 1** Module and formatter arms (closes #379)
  - [ ] `dtl.rs`: `bits()`, `filter_expression()`, `FilterExpression`, `Filter`
  - [ ] djade lexer cases as table tests, regex oracle test
  - [ ] `-stmt` and `-expr` arms in `format.rs`, Django only, fast paths
  - [ ] `tests/fmt/django/syntax_sanity.{html,snap}` from `pj-rebase` (`9f89110`)
  - [ ] Migrate `untrimmed_blocktranslate`, `duplicate_block_name`, `same_file_partial_include`
  - [ ] Gates: ecosystem diff ≈ 51 lines over 36 files, `just bench-rs` flat, `just ecosystem-check-stability djade` green
- [ ] **Step 2** Checker plumbing and #501 rebase
  - [ ] Route `JinjaTagOrChildren::Tag` through `visit_jinja_tag`
  - [ ] `visit_jinja_interpolation`
  - [ ] Rebase #501: keep the `upgrade` category and both rules, drop `OLDEST_SUPPORTED` and the comp-then-base mode
  - [ ] `redundant-json-script-id` rewritten on `filter_expression()`
- [ ] **Step 3** `tags_in_text`
- [ ] **Step 4** `trans` / `blocktrans` rename
- [ ] **Step 5** `endblock` / `endpartialdef` labels
- [ ] **Step 6** One PR each
  - [ ] Legacy `as`
  - [ ] `ifequal` / `ifnotequal`
  - [ ] `length_is`
- [ ] **Step 7** markup_fmt fork: `{# #}` edge spacing, blank line between top-level blocks
- [ ] **Step 8** Docs: known limitations, "coming from djade" map

## Steps

### Step 1: Module and formatter arms

**Files**

- `crates/djangofmt_lint/src/dtl.rs` (new), exported from `lib.rs`
- `crates/djangofmt_lint/Cargo.toml`: `regex` under `[dev-dependencies]`
- `crates/djangofmt/src/commands/format.rs`: two arms before the `_ => Ok(code.into())` fallback (`:421`)
- `crates/djangofmt/tests/fmt/django/syntax_sanity.{html,snap}`: cherry-pick from `pj-rebase` (`9f89110`); expected output is unchanged
- The three rules listed under "Migrations"

**`bits(content: &str) -> Vec<&str>`**

Django's `smart_split_re` (`django/utils/text.py`, unchanged since 2010):

```
((?:[^\s'"]*(?:(?:"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')[^\s'"]*)+)|\S+)
```

Rules the scanner must keep:

- Whitespace is Python's `\s`: `char::is_whitespace()` plus U+001C to U+001F.
- A quoted run (`"…"` or `'…'`, backslash escapes inside) is glued to the non-whitespace characters touching it: `x"a b"y` is one bit, so is `trans"Move  Account"`.
- An unterminated quote falls back to `\S+`: `"a b` is two bits, `"a` and `b`.
- Quotes stay in the slice. No `_("…")` re-join (`split_contents` does that; no planned rule needs it and it never changes `join(" ")`).

**`filter_expression(expr: &str) -> Option<FilterExpression<'_>>`**

```rust
pub struct FilterExpression<'a> { pub var: &'a str, pub filters: Vec<Filter<'a>> }
pub struct Filter<'a> { pub name: &'a str, pub arg: Option<&'a str> }
```

Django's `filter_re` (`django/template/base.py`; copy the exact pattern from the pinned Django release):

```
constant = (?:_\("…"\)|_\('…'\)|"…"|'…')    where "…" = "[^"\\]*(?:\\.[^"\\]*)*"
num      = [-+\.]?\d[\d\.e]*
var      = [\w\.]+ | num
^(?P<constant>constant) | ^(?P<var>var)
| (?:\s*\|\s*(?P<filter_name>\w+)(?::(?:(?P<constant_arg>constant)|(?P<var_arg>var)))?)
```

Semantics to reproduce from `FilterExpression.__init__`:

- Matches must tile the whole input with no gap. Django raises on `upto != start` and on `upto != len(token)`; both return `None` here.
- The head is a constant or a variable. `var` keeps the slice as is, so `_("x")` stays inside `var`.
- `\w` is Python's: alphanumeric or `_`. Use `char::is_alphanumeric() || c == '_'`. Known divergence from Django on U+0345 only.
- `None` for `super()`, `engines[0].name|length`, `a .b`, `?x`: 133 corpus variables are invalid and must stay verbatim.

**Formatter arms** (inside the closure in `format_text`; `profile` is in scope)

```rust
"markup-fmt-jinja-stmt" if profile == Profile::Django => {
    // Fast path: a trimmed body with no run of whitespace and no tab/newline is already normalized.
    Ok(if is_single_spaced(code.trim()) { code.into() } else { bits(code).join(" ").into() })
}
"markup-fmt-jinja-expr" if profile == Profile::Django => {
    let expr = code.trim();
    Ok(match filter_expression(expr) {
        Some(fe) if expr.contains(char::is_whitespace) => fe.to_string().into(), // var|name:arg…
        _ => code.into(),
    })
}
```

- markup_fmt already trims the body, wraps it with one space on each side and, in Django mode, joins lines with `collapse_django_ws` (fork `printer.rs:2308`). The arms only decide inner spacing.
- The `-expr` arm also receives `{{ }}` inside attribute values (fork `printer.rs:2981`). `{% %}` inside attribute values never reaches the closure; that stays a known limitation.
- No `catch_unwind`, no config clone: both functions are total and allocation-light.

**Tests**

- Port djade's lexer tests (`src/main.rs`, the `split_contents` and `lex_filter_expression` tests) as table tests.
- Oracle test: port the two regexes on the `regex` crate, generate 200k random strings per function from a seeded xorshift64* over a template-ish alphabet (`a-z 0-9 _ . | : " ' \ space tab newline - + ( ) %` plus a few non-ASCII letters), assert `bits()` equals the regex `find_iter` and `filter_expression()` equals the regex tiling. Exclude U+001C to U+001F from the alphabet (Rust's `\s` differs from Python there) and cover them in a table case.
- Snapshot: `django/syntax_sanity` from `pj-rebase`.

**Migrations** (snapshots must stay identical)

- `untrimmed_blocktranslate.rs:70-74`: `bits(content).iter().any(|b| *b == "trimmed")`.
- `duplicate_block_name.rs:93` `block_name_from_content`: `bits()` after the `+`/`-` trim. `block_names_in_text` waits for step 3.
- `same_file_partial_include.rs:104-130` `parse_partial_include`: `bits()` then strip the quotes of bit 1.
- `missing_doctype.rs:80` only compares the tag name from `parse_jinja_tag_name`; nothing to migrate.

**Gates**

- `just ecosystem-check-dev` on the Django-profile projects: expect the 51 changed lines over 36 files `pj-rebase` produced, all inner-token or filter spacing.
- `just bench-rs`: no regression. Both arms were measured at about 1% of formatting the corpus.
- `just ecosystem-check-stability djade` stays green.
- Close #379 from the PR description.

Commit: `feat(format): Normalize whitespace inside Django tags and filter expressions`

### Step 2: Checker plumbing and #501 rebase

**Checker** (`crates/djangofmt_lint/src/checker.rs`)

- `visit_jinja_block` (`:313`) only descends into `JinjaTagOrChildren::Children`. Also call `visit_jinja_tag` on every `JinjaTagOrChildren::Tag`, so openers, `else`/`elif` and closers reach single-tag rules. `{% include %}` is never a block tag, so `same_file_partial_include` is unaffected, and `missing_doctype` only reads `extends`. Assert every existing snapshot is unchanged.
- Add `visit_jinja_interpolation(&JinjaInterpolation)` from #501, called from `visit_node` on `NodeKind::JinjaInterpolation`. `is_django()` is already on main (`:62`).

**#501 rebase** (`feat/lint-upgrade-rules`)

- Keep: the `upgrade` category in `registry.rs` and `rules/mod.rs`, `deprecated-static-library` (gated 2.1, already deletes the alias when `static` is loaded), the `redundant-json-script-id` fixtures, the `.jinja` valid fixtures.
- Drop: `DjangoVersion::OLDEST_SUPPORTED` and the unset-to-5.2 resolution in `settings.rs`; the comp-then-base mode (`justfile`, `python/ecosystem-check/ecosystem_check/format.py`).
- Rewrite `redundant-json-script-id` on `filter_expression(interp.expr)`: the filter named `json_script` whose `arg` is `""` or `''`. The edit deletes from the `:` to the end of the arg (`source_span(arg)` widened by one byte). Gated 4.1. A string literal containing `|json_script` can no longer false-positive.

**Gates**: fixtures; `just ecosystem-check-lint-dev` with zero hits, since the corpus has none for either rule.

### Step 3: `tags_in_text`

- Generalise `block_names_in_text` (`duplicate_block_name.rs:73-89`) into `helpers::tags_in_text(text: &str) -> impl Iterator<Item = JinjaTag<'_>>` yielding `JinjaTag { content, start }` with `start` relative to `text`; the checker adds the text's source offset. Mirror Django's `tag_re`: `{%` to the next `%}` on the same line, no DOTALL. `{{ }}` and `{# #}` are skipped.
- Call sites in the checker: HTML comments, the raw bodies of `script`/`style`/`pre`/`textarea`, and `NativeAttribute.value` when it contains `{%`. Each synthesized tag goes through `visit_jinja_tag`.
- Opt-out: a `TEXT_SENSITIVE_RULES` list beside `RAW_SENSITIVE_RULES` (`checker.rs:22`) for rules that must not fire on text tags. Decide per rule while wiring; the default is to see them.
- `duplicate_block_name` switches to the shared scan.

**Gates**: `duplicate-block-name` snapshots unchanged; a new fixture with `{% trans %}` inside `placeholder="…"` and inside `<script>`.

### Step 4: `trans` / `blocktrans` rename

- Rule in `upgrade`, gated at 3.1 (`translate` appeared in 3.1; Django 6 still registers both names, so this is a preference, not a deprecation).
- `visit_jinja_tag`, which now covers standalone tags, block openers and closers, and text tags. Bit 0 renames: `trans` to `translate`, `blocktrans` to `blocktranslate`, `endblocktrans` to `endblocktranslate`. On `load … from i18n`, rename the `trans`/`blocktrans` items (djade `migrate_translation_tags`, `main.rs:508`).
- Edit: replace that one bit's span. Safe fix.
- Overlap with `untrimmed-blocktranslate` on the same opener: the applier skips the second overlapping fix and re-lints, up to `MAX_FIX_ITERATIONS` (10). Add a fixture with both diagnostics on one opener and assert the fixed snapshot carries both changes. Reach for `IsolationLevel::Group` only if the loop does not converge.
- Corpus: 14,122 hits (12,498 in the AST, 1,313 in attribute values, 311 in raw bodies).

### Step 5: `endblock` / `endpartialdef` labels

- One rule in `style`, ungated. `visit_jinja_block`: opener bit 0 is `block` or `partialdef` with the name in bit 1 (`partialdef name inline` keeps the name in bit 1); closer bit 0 is `endblock` or `endpartialdef`.
- Different lines and a one-bit closer: insert ` name` after bit 0. Same line and a two-bit closer: delete the label. Safe fixes. The line of an offset comes from counting newlines up to `tag.start`, or from the line index the lint context already keeps.
- Idempotency: decided from source lines like djade; stable when fixes run after formatting (adding a label lengthens a closer that already sits on its own line, dropping one shortens a one-line block).
- Corpus: 7,154 added, 618 dropped, 36 on `endpartialdef`.

### Step 6: Legacy `as`, `ifequal`, `length_is`

One PR each, djade's inline tests ported as `invalid`/`valid` fixtures.

**Legacy `as`** (`upgrade`, ungated). `visit_jinja_tag` on `with` and `blocktranslate`/`blocktrans` openers. Port `migrate_assignments_with_tag` (`main.rs:650`) and `migrate_assignments_blocktranslate_tag` (`:676`): `with a as b`, `and`-separated pairs, mixed with `k=v`, `count n as m` in blocktranslate. `{% trans "x" as var %}` is `asvar` and is never touched. The rewrite rebuilds the bit list, so the edit replaces the opener body. 22 of the 672 corpus tags have a quoted operand with a space, which is why this needs `bits()`.

**`ifequal` / `ifnotequal`** (`upgrade`, gated 3.1). `visit_jinja_block`. An opener with exactly three bits `ifequal a b` becomes `if a == b` (`!=` for `ifnotequal`); the closer becomes `endif`. One `Fix` with two edits. Skip any other arity, as djade does (`migrate_ifequal_tags`, `:544`).

**`length_is`** (`upgrade`, gated 4.2). `visit_jinja_block` on an `if` opener with exactly two bits whose second bit matches, anchored on the whole bit, `^[\w.]+\|length_is:\w+$`, rewritten to `X|length == N`. Anchoring avoids djade's `a|default:b|length_is:1` to `b|length` bug (`migrate_length_is`, `:465`). Only `if`, never `elif`.

### Step 7: markup_fmt fork

Branch on `django/baseline` of `UnknownPlatypus/markup_fmt`, pin the rev in the root `Cargo.toml`, cherry-pick onto `g-plane/main` for the upstream PRs.

- Split `format_comments` so template comments format independently of HTML comments; djangofmt enables template comments in Django mode: `{#choo choo#}` becomes `{# choo choo #}`. Assert `{# djangofmt: ignore[...] #}` directives still round-trip and still suppress.
- Root printer, Django mode, only when a top-level `extends` exists: exactly one blank line between consecutive top-level blocks. Today two or more collapse to one and a missing one is never inserted.

**Gates**: snapshot tests; `just ecosystem-check-stability djade` green; ecosystem diff reviewed by hand.

### Step 8: Docs

- `docs/known-limitations.md`: `{% load %}` tags are never merged or sorted (djade does both, so djangofmt-then-djade differs there by design); `{% %}` inside attribute values is not normalized by the formatter, lint rules still see them.
- `docs/linter.md` or the README: a short "coming from djade" map of which djade behaviour lives in which rule or formatter behaviour.

## Out of scope

- Jinja expression formatting (#79) and pretty_jinja. `pj-rebase` holds the last working integration; its path dependency has to become a git pin before it can build in CI.
- `load` merging and sorting. A later `style` rule could enforce one library per `{% load %}`, the opposite of djade.
- The reverse parity mode (djangofmt then djade).
- `{% %}` inside attribute values in the formatter; markup_fmt does not route them.
- PR #209 (Django block whitespace sensitivity) is unrelated.

## Reference

- djade `src/main.rs` on `main` (1.9.0): `split_contents` :351, `lex_filter_expression` :275, `format_variable` :438, `migrate_static_load_tags` :608, `migrate_empty_json_script` :486, `migrate_translation_tags` :508, `migrate_length_is` :465, `migrate_ifequal_tags` :544, `migrate_assignments_with_tag` :650, `migrate_assignments_blocktranslate_tag` :676, `update_endblock_and_endpartialdef_labels` :866.
- Django: `django/utils/text.py` `smart_split_re`; `django/template/base.py` `filter_re` and `FilterExpression.__init__`; `django/templatetags/i18n.py` registers both `trans` and `translate`.
- Corpus (17,584 Django templates, 39 ecosystem repos plus greenday): trans 14,122 · labels 7,808 · legacy `as` 672 · comment spacing 539 · filter spacing 308 · inner-token spacing 107 · the four removed-syntax fixers 0.
- Process: `.agents/skills/add-lint-rule/SKILL.md` for every rule PR; `just pre-mr-check` before each landing.

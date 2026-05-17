# djangofmt-lint — Roadmap to "Ruff for Django Templates"

> Status: **draft** — strategic plan, not committed scope. Updated 2026-05-17.

## TL;DR

djangofmt-lint should become a **thin orchestrator** that bundles HTML lint
rules from [markuplint](https://github.com/markuplint/markuplint) (via a
new Rust parser plugin we contribute) and Django-template lint rules from
[django-language-server](https://github.com/joshuadavidthomas/django-language-server)
(djls) — exposing them under one CLI, one config, and one autofix loop.
Native rules in djangofmt-lint are limited to the (small) set of truly
interleaved Django+HTML checks plus formatter-integration concerns.

We reimplement nothing we don't have to. We coordinate with upstream
maintainers before writing parser code. We ship a useful local v0.3
*today* so we're never blocked on coordination.

---

## Goals

1. **One tool, one CLI.** `djangofmt check` runs HTML + Django + interleaved
   rules and presents one diagnostic stream, one autofix loop, one config.
2. **Minimise reimplementation.** Markuplint already has 43+ HTML rules with
   ARIA, content-model, and accname algorithms; we use those, not reimplement
   them. djls already has 24+ Django rules with deep semantic context; we
   reuse them where their API permits.
3. **Contribute, don't fork.** Where upstreams are missing infrastructure
   (Rust parser-plugin contract, published Django parser crate), we propose
   PRs upstream rather than vendoring.
4. **Ship continuously.** We deliver a useful local-rules version of v0.3
   before upstream coordination concludes, so adoption isn't blocked.

## Non-goals

- Replacing djls. It's an LSP server, optimized for incremental editor
  feedback via salsa. djangofmt-lint is a CLI-first tool. Their architectures
  are correctly different for their jobs.
- Replacing markuplint. We're a downstream consumer of its rule engine
  for HTML rules.
- Asking djls to refactor its rule layer into pure functions.
  That refactor would damage their LSP latency profile.
- A general-purpose Django language server. (See djls.)
- Project-wide semantic context (INSTALLED_APPS, custom tag introspection,
  model graphs). All rules are file-local at first.

---

## Architecture

```
Django template source (e.g. base.html)
            │
            ├──► markup_fmt parser ─► markup_fmt AST ─► djangofmt (formatter)
            │                                              │
            │                                              │
            │                                              ▼
            │                              djangofmt-lint native rules
            │                              (interleaved Django+HTML rules
            │                               + Django-specific rules if djls
            │                               unavailable)
            │
            ├──► markuplint-django-parser (new, contributed) ─► MLAST ─► markuplint-rules
            │                                                                    │
            │                                                                    ▼
            │                                                       HTML/A11y/ARIA rules (43+)
            │
            └──► djls-templates parser (when published) ─► NodeList ─► djls rule functions
                                                                            │
                                                                            ▼
                                                                Django-only rules (S100-S123)

                  All three rule sources feed into one diagnostic stream:
                                          │
                                          ▼
                          djangofmt-lint: unified output,
                          unified autofix loop, unified config
                                          │
                                          ▼
                                  djangofmt check (CLI)
                                  djangofmt-lsp (LSP wrapper, later)
```

### Why two parsers over the same source

Both `markup_fmt` and `markuplint-django-parser` need to parse the template.
Markup_fmt drives formatting (its AST is shaped for printing); MLAST drives
markuplint's HTML rule engine (shaped for HTML+DOM analysis). Reusing one
for the other would require either rewriting markuplint's rule engine
against a different AST or rewriting djangofmt's formatter against MLAST.
Both are far more invasive than running two parsers per file. Parse cost
is dwarfed by IO in a CLI tool.

### Where djangofmt-lint's native rules live

Three buckets:

| Bucket                      | Examples                                                                                                                   | Why native                                                                                         |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| **Interleaved Django+HTML** | `{% url %}` inside `href`, `safe` inside attr, `{% if %}` straddling attr boundary                                         | Neither MLAST (HTML-shaped) nor djls's flat NodeList (Django-only) can see both sides. ~3–5 rules. |
| **Formatter-integration**   | Rules that ensure lint and formatter agree (e.g., `djangofmt:ignore` directives, custom-block discipline)                  | Tied to formatter behavior; nobody else owns the formatter.                                        |
| **Fallback Django rules**   | If djls doesn't publish a usable crate, we keep small native versions of `extends-must-be-first`, `multiple-extends`, etc. | Stop-gap; retire if/when djls integration lands.                                                   |

---

## Phases

### Phase 0 — Open upstream conversations (now, ~half a day)

Before writing code. Two GitHub discussions:

- **markuplint/markuplint** — see Appendix A.
- **joshuadavidthomas/django-language-server** — see Appendix B.

No code blocked on the responses. Reading-only deliverable: the two posts.

### Phase 1 — Ship local-rules v0.3 (~2–3 weeks)

Goal: working tool with ~10 rules, no Rust dependencies we don't already
have. Doubles as a proof artifact for the upstream discussions.

#### 1.1 `djangofmt_html_spec` crate

Vendor [`@markuplint/html-spec`](https://github.com/markuplint/markuplint/tree/v6/packages/%40markuplint/html-spec)
JSON (MIT, 1.9 MB) into a new crate.

- `crates/djangofmt_html_spec/` skeleton.
- `build.rs` that pulls `index.json` from a pinned commit (committed to repo;
  refreshed via a separate script, not at every build).
- License attribution: `LICENSE` file + `NOTICE` line.
- `pub trait HtmlSpec` — the swap surface for future `markuplint-types`
  dependency:
  ```rust
  pub trait HtmlSpec {
      fn element(&self, name: &str) -> Option<&ElementSpec>;
      fn is_void(&self, name: &str) -> bool;
      fn is_deprecated_attr(&self, el: &str, attr: &str) -> bool;
      fn allowed_attr_values(&self, el: &str, attr: &str) -> Option<&[String]>;
      // ... extended as rules require
  }
  ```
- `MarkuplintHtmlSpec` impl that deserializes the vendored JSON via serde
  on first use, cached in `OnceLock`.

#### 1.2 Refactor `rules/` by category

Mirror Ruff's layout:

```
crates/djangofmt_lint/src/rules/
  ├── a11y/
  ├── correctness/
  ├── style/
  ├── suspicious/
  └── helpers.rs
```

The `define_rules!` macro already supports any path; only file layout
changes. Update `RuleCategory` enum to match.

#### 1.3 Native rules — first pass

HTML (via `HtmlSpec`):

- `img-missing-alt` (a11y, safe-fix when adjacent text exists, unsafe-fix `alt=""`)
- `deprecated-attr` (style, safe-fix = remove)
- `deprecated-element` (style, no fix)
- `label-has-control` (a11y, no fix)
- `invalid-attr-value` (existing — rewire through `HtmlSpec`, drop hardcoded `<form method>`)

Django (file-local; no project context):

- `extends-must-be-first` (correctness, no fix — code `S122` aligned with djls)
- `multiple-extends` (correctness, no fix — code `S123`)
- `load-before-tag` (correctness, safe-fix = reorder)
- `unknown-block-name` (suspicious, no fix)
- `blocktranslate-no-trimmed` (existing — keep)

#### 1.4 CLI alignment

- Add `--select`/`--ignore` parsing (ruff-style codes).
- Add `[tool.djangofmt.lint]` section to pyproject.toml parser.
- Document the rule catalogue in `README.md` / new `docs/rules/`.

#### 1.5 Ship `djangofmt 0.3.0`

Cut release. Reference the release in the upstream discussions
("here's the tool consuming your work").

### Phase 2 — Markuplint integration (3–6 weeks, depends on Phase 0 response)

Three paths depending on maintainer signal:

#### Path 2A — Maintainer accepts parser plugins now

1. **Land the upstream prerequisites** (per markuplint discussion):
   - `Serialize` derive on `markuplint-core::mlast::*` types.
   - `markuplint_rules::lint::lint_from_mlast(&MLASTDocument, &spec, &config) -> Vec<Violation>`
     entry point.
   - Documented parser-plugin contract for Rust parsers.
2. **Write `markuplint-django-parser`** (~2–3 weeks):
   - Reference parsers: `nunjucks-parser` (TS, closest semantically) and
     `markuplint-html-parser` (Rust precedent).
   - Map Django constructs to MLAST:
     - `{% tag %}` blocks (with children) → `MLASTPSBlock`
     - `{{ expr }}` → `MLASTPSBlock` (no children)
     - `{# comment #}` → `MLASTComment`
     - HTML between → standard MLAST elements
   - UUID pairing for `pair_node_uuid`.
   - Span / line / col tracking.
3. **Integrate in djangofmt-lint**:
   ```rust
   let mlast = markuplint_django_parser::parse(source)?;
   let violations = markuplint_rules::lint::lint_from_mlast(&mlast, &spec, &config)?;
   ```
4. **Translate** their `Violation` → our `LintDiagnostic` (line/col → byte
   span via existing `LineIndex`).
5. **Retire** the 5 native HTML rules from Phase 1.3 in favor of upstream.
6. **Result**: all 43+ markuplint rules now run on Django templates.
   New rules from markuplint upstream flow in via `cargo update`.

#### Path 2B — Maintainer says "after v6 ships"

1. Keep the native HTML rules from Phase 1. Deepen them; port more from
   markuplint's TS source manually (small set, 5–10 rules).
2. Track v6 release in renovate / a watcher script.
3. Resume Path 2A when v6 stable lands.

#### Path 2C — Maintainer declines parser plugins

1. **Local adapter pattern**: write `markup_fmt::Root → MLASTDocument`
   converter inside djangofmt-lint, never upstreamed.
2. Pull `markuplint-rules` as a git-path dep (acceptable since it's MIT).
3. Same end result rule-wise, but **no contribution upstream and no benefit
   to the ecosystem**. Last-resort path.

### Phase 3 — Djls integration (parallel with Phase 2)

Same three-path structure.

#### Path 3A — djls publishes `djls-templates` on crates.io

1. Depend on the published crate.
2. For each Django rule we'd otherwise port, instead extract from djls's
   `validate_nodelist` what we need. (Their rules touch salsa accumulators,
   so we wrap or copy the pure-logic kernel — small per rule.)
3. **Align diagnostic codes**: `S100–S123` match djls. Users running
   djls in their editor and djangofmt in CI see the same codes.

#### Path 3B — djls doesn't publish, but aligns codes

Keep our native Django rules. Coordinate codes only. Lower ROI but trivial
maintenance cost.

#### Path 3C — djls doesn't engage

Keep native Django rules. Don't align codes (or use our own with documentation
linking djls codes for users who want both tools).

### Phase 4 — The "ruff" surface (1–2 weeks, after Phase 1 ships)

Where djangofmt-lint becomes a meta-linter, not just a linter.

#### 4.1 Unified config

```toml
[tool.djangofmt.lint]
select = ["A11Y", "E", "S"]      # ruff-style category codes
ignore = ["A11Y013"]
unsafe-fixes = false

[tool.djangofmt.lint.per-file-ignores]
"templates/admin/*.html" = ["A11Y"]
```

Categories namespace the three rule sources:

- `A11Y*` — markuplint a11y rules
- `H*` — markuplint HTML correctness/style rules
- `S*` — Django structural rules (djls or our fallback)
- `E*` — interleaved/native djangofmt-lint rules

#### 4.2 Unified diagnostic stream

All three sources emit into our `LintDiagnostic` shape. Markuplint
`Violation` and djls `ValidationError` get translation layers
(~50 lines each). No leakage of upstream types into rule registry / CLI.

#### 4.3 Unified autofix loop

Our existing `lint_fix` cascade handles fixes regardless of source.
Markuplint upstream doesn't have an autofix model; ours wraps theirs at
the diagnostic level. Each rule integrator decides whether the upstream
diagnostic comes with an actionable fix (most won't, for now).

#### 4.4 Rule-code mapping documentation

Generate `docs/rules/index.md` from the registry. Each rule entry:

- our code
- upstream code (if borrowed)
- upstream link (markuplint rule doc, djls rule doc)
- one-line description
- fix availability
- example

### Phase 5 — Give back (ongoing)

Areas where djangofmt-lint can be donor in shared crates:

| Component                                        | To whom                                   | When                        |
| ------------------------------------------------ | ----------------------------------------- | --------------------------- |
| `Edit` / `Fix` / `Applicability` autofix model   | Markuplint (when they reach code-actions) | After Phase 2 stabilizes    |
| `HtmlSpec` trait shape (refined by real callers) | markuplint-types                          | Once 10+ rule callers exist |
| `markuplint-django-parser`                       | markuplint upstream                       | Phase 2A                    |
| LSP wrapper learnings                            | djls (cross-tool diagnostic compat)       | After Phase 4               |
| Diagnostic schema convention                     | community / lsp-types?                    | Far future                  |

---

## Risk register

| Risk                                              | Probability | Impact                           | Mitigation                                                                      |
| ------------------------------------------------- | ----------- | -------------------------------- | ------------------------------------------------------------------------------- |
| Markuplint maintainer says "wait until v6 stable" | Med         | High (delays Phase 2A by months) | Phase 1 keeps shipping; native rules cover gap                                  |
| MLAST shape changes during v6                     | Med         | Med (forces parser updates)      | Pin specific commit rev; `HtmlSpec` trait insulates rule code                   |
| Django MLAST parser is 4+ weeks not 2             | High        | Med                              | Scope first iteration to minimum viable; iterate                                |
| djls maintainer doesn't publish                   | Med         | Low                              | Native Django rules are small set; doable                                       |
| Bikeshed on upstream API design                   | Med         | Med                              | Ship native code first, propose extracted APIs only after working callers exist |
| Markuplint declines parser plugins                | Low         | Med                              | Path 2C (local adapter) achieves same rules, just less virtuous                 |
| html-spec JSON drifts faster than we can refresh  | Low         | Low                              | Pin and refresh quarterly; codegen has its own cadence                          |

---

## Open questions

1. **Where do error codes prefix from?** Ruff: `E*` / `W*` / `F*`. Our
   three sources need namespacing. Proposed: `A11Y*`, `H*`, `S*`, `E*`.
   Should we pick now or after Phase 1 to see real distribution?
2. **License/attribution for vendored JSON spec data.** MIT permits but
   we should add `NOTICE` + maintainer credit explicitly. Done in Phase 1.1.
3. **Should `djangofmt_html_spec` ever be published to crates.io?**
   Probably not — it duplicates upstream data; once `markuplint-types`
   becomes a real dep, this crate becomes a thin shim and we collapse it.
4. **Per-file-ignores resolution order.** Match ruff's semantics
   (later config overrides earlier). Document explicitly.
5. **Performance budget.** 50 ms p50 / 200 ms p95 per template feels
   right. Bench in Phase 1.5.
6. **Should we publish `djangofmt-lsp` standalone or as a feature flag?**
   Decide in Phase 4. Probably standalone — independent versioning.

---

## What I'd do this week

1. **Open the two upstream discussions** (Appendices A & B).
2. **Scaffold `crates/djangofmt_html_spec/`** with the vendored JSON.
3. **Port `deprecated-attr` and `img-missing-alt`** through the new
   `HtmlSpec` trait. These two validate the trait shape before commitment.
4. **Track responses** to upstream discussions; budget 2 days for
   substantive replies before stretching scope.

---

# Appendix A — Markuplint upstream discussion draft

> **Action**: post as a GitHub discussion on `markuplint/markuplint`,
> category "Ideas" or "Q&A". Cross-reference v6 branch.
> Adjust tone & details as the actual codebase changes between draft and post.

---

**Title**: *Rust parser-plugin contract — proposing `markuplint-django-parser` as a contribution*

Hi! I maintain [djangofmt](https://github.com/UnknownPlatypus/djangofmt),
a Rust-native formatter and linter for Django templates. I've been
following the v6 Rust port closely — congratulations, the work landing in
`crates/` is impressive, especially the WHATWG-conformant parser and the
ARIA algorithms.

I'd like to discuss whether contributing a **`markuplint-django-parser`**
crate to markuplint would be welcome, and what the right way to structure
the contribution would be.

## Context

djangofmt today uses [`markup_fmt`](https://github.com/g-plane/markup_fmt)
for parsing and formatting Django/Jinja templates. We've built a small
file-local lint framework on top of its AST, mirroring Ruff's design
(applicability tiers, autofix loop, registry macro). It works, but I want
to avoid reimplementing HTML rules that already exist in markuplint —
particularly the deeper ones (`wai-aria`, `permitted-contents`,
`label-has-control`, the ARIA accname algorithm).

Looking at markuplint v6, the natural way to consume those rules from a
Rust CLI is to produce MLAST in Rust and feed it to `markuplint-rules`.
There's clear precedent for parser-pluggability in the TS packages
(`nunjucks-parser`, `liquid-parser`, `erb-parser`, etc.) and `MLASTPSBlock`
seems to be exactly the right primitive for `{% %}` / `{{ }}` constructs.

## What I'd like to contribute

A new crate `crates/markuplint-django-parser/` that:

- Produces `MLASTDocument` directly in Rust from Django template source.
- Maps `{% tag %}` blocks with children → `MLASTPSBlock` (with
  `child_nodes`); `{{ expr }}` → leaf `MLASTPSBlock`; `{# comment #}` →
  `MLASTComment`.
- Handles UUID pairing for `pair_node_uuid`, namespaces (SVG, MathML for
  embedded SVG/MathML in Django templates), span/line/col tracking.
- Uses `markuplint-html-parser`'s tokenizer where appropriate (for the HTML
  portions between Django constructs).
- Is shaped after `nunjucks-parser` (TS) — Nunjucks is Jinja-derived, so
  the Django case is a small delta on top.

I'd be happy to write this and submit it as a PR series.

## Three blockers I'd like to discuss before starting

These are the only things stopping me from drafting the parser today:

1. **`Serialize` on MLAST types.** `markuplint-core::mlast::*` derives
   `Deserialize` only. A Rust parser today would have to either roundtrip
   through JSON (gross) or have an in-process entry point to the rule
   engine. I propose:
   - Adding `Serialize` to the MLAST types (forward-compatible).
   - Adding a `lint_from_mlast(&MLASTDocument, &MLMLSpec, &LintConfig) -> Vec<Violation>`
     entry point alongside the JSON-accepting `lint`.

2. **Rust parser-plugin contract.** `markuplint-html-parser` is currently
   the only Rust parser and it's tightly coupled to `markuplint-builder`'s
   napi surface. Is there appetite for documenting a Rust parser-plugin
   contract (similar to how `parser.ts` is documented for TS parsers)?
   I can draft this contract along with the parser PR if it'd help.

3. **Timeline / v6 stability.** Is v6 stable enough for parser authors to
   write against today, or would you prefer parser contributions wait
   until after v6.0 ships? I can either build now and track API churn, or
   wait and contribute against the stable release — happy to do whichever
   you prefer.

## What's in it for markuplint

- A first-class Django template story (Django is one of the top three Python
  web frameworks; ~1.4M monthly downloads).
- A test case for the parser-plugin contract that exercises Rust-native
  parsing end-to-end, before more parsers want to follow.
- A concrete downstream user driving feedback on `markuplint-types` /
  `markuplint-rules` Rust API ergonomics.

## What I'm not asking for

- I'm not asking you to rewrite anything to fit djangofmt. The parser
  should fit your existing patterns.
- I'm not asking for djangofmt-specific features in the rule engine. Our
  Django-specific rules live in djangofmt, not here.
- I'm not asking for autofix infrastructure (though djangofmt has one we'd
  be happy to discuss contributing separately, if/when markuplint reaches
  the code-actions milestone).

Happy to chat sync, async, in any format. Thanks for the great work on v6 —
the rule engine is the kind of substrate the ecosystem has been missing.

— Thibaut

---

# Appendix B — django-language-server upstream discussion draft

> **Action**: post as a GitHub discussion on
> `joshuadavidthomas/django-language-server`, category "Ideas".
> Adjust tone & specifics as djls evolves between draft and post.

---

**Title**: *Coordinating with djangofmt: publishing `djls-templates`, aligning diagnostic codes*

Hi Josh! I maintain [djangofmt](https://github.com/UnknownPlatypus/djangofmt),
a Rust-native formatter and linter for Django templates targeted at CLI /
pre-commit workflows. I've been watching djls develop and I think there's
a productive way for the two tools to coexist and complement each other —
and I want to start that conversation before going off and reimplementing
things you've already done well.

## Where I see the split

djls is an LSP — incremental, salsa-tracked, project-aware (templatetag
introspection, INSTALLED_APPS, model graphs). The architecture is right
for editor latency.

djangofmt is a CLI — parallel-per-file, file-local, autofix-loop driven.
Different problem, different architecture.

Users want both: djls in their editor, djangofmt in pre-commit and CI.
Today they'd see different rule codes, different output formats, different
diagnostics. We can do better.

## Two small asks

1. **Would you consider publishing `djls-templates` to crates.io with a
   stable API?**

   djls-templates' flat `Node::{Tag, Variable, Comment, Text}` is a clean,
   minimal Django parser. djangofmt's main parser
   ([markup_fmt](https://github.com/g-plane/markup_fmt)) handles HTML + Jinja
   interleaved, which is what we need for formatting and for our small
   set of cross-cutting rules. But for Django-only rules (extends placement,
   tag arity, blocktranslate-trimmed), djls-templates' flat shape is exactly
   right.

   If `djls-templates` were a published crate, djangofmt could:
   - Run djls-templates over the same source as a second parser for
     Django-only rules.
   - Reuse the parser logic instead of forking it.
   - Track upstream parser improvements automatically.

   Would this be feasible? What's the API stability picture from your side?

2. **Could we align diagnostic codes?**

   djls has codes `S100–S123` for its validation errors. djangofmt has
   started shipping similar rules (`extends-must-be-first`, `multiple-extends`,
   `blocktranslate-no-trimmed`) and is about to ship more. I'd like our codes
   to *be* yours where we cover the same checks — so a user running djls
   in their editor and djangofmt in pre-commit sees the same `S122` for the
   same problem.

   Specifically I'm thinking:
   - Coordinate via a shared markdown file (in either repo) that lists each
     `S###` code, its semantics, and which tool(s) implement it.
   - Treat the file as the source of truth when either tool adds a new
     Django-template structural check.

   No formal coupling, no shared crate required — just a coordination
   document.

## Things I am explicitly NOT asking

- I'm not asking djls to extract rules into pure functions. Your salsa
  accumulators are right for an LSP; refactoring them would compromise the
  thing djls does best.
- I'm not asking djls to take on autofix. djangofmt's CLI is the natural
  home for that; we can offer code-action protocol support separately if
  useful for editors.
- I'm not asking for project-wide Django introspection in djangofmt.
  That's clearly your turf and rightly so.

## What I'd offer in return

- Ongoing rule-by-rule code alignment as we add checks.
- Diagnostic-format compat: djangofmt-lint can emit `lsp_types::Diagnostic`
  via a wrapper crate. Editor extensions that want a CLI-driven fallback
  could use it.
- Cross-linking in our docs / READMEs once a coordinated approach is in
  place.
- A test corpus of Django templates we both run against, if useful for
  cross-validation.

Happy to chat in any format. Thanks for djls — the Django-template
ecosystem has been thin on real tooling for too long.

— Thibaut

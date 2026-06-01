> # ⚠️ WIP — NEEDS REFINING (do not implement as-is)
>
> This is an unfinished draft, committed as WIP to continue later. Before implementing:
>
> 1. **Parent/ancestor context handling is not designed properly yet.** The Architecture section
>    proposes bolting rule-specific fields (`raw_text_depth`, `translated_depth`) onto the shared
>    `Checker` struct. That is slop — the generic visitor must not carry state that exists only to
>    serve one rule. Design a **general** mechanism for querying ancestor context during traversal
>    (e.g. the `Checker` maintains a generic ancestor stack of element tag names / node kinds that
>    any rule can consult, or context is threaded through the visit methods), then express "skip
>    text inside `script`/`style`/`pre`/`textarea` and inside `{% blocktranslate %}` bodies" in
>    terms of that general facility. Revisit how text-node visiting and sibling lookup hang off it.
> 2. Everything below is otherwise a first draft and open to revision (scope, category, fixtures).

---

# Plan: `untranslated-text` lint rule (djangofmt #217)

## Context

[Issue #217](https://github.com/UnknownPlatypus/djangofmt/issues/217) asks djangofmt to find raw
literal text in Django templates that isn't wrapped in i18n tags (`{% translate %}` /
`{% blocktranslate %}`) and ideally rewrite it. Prior art [jkaeske/django-i18n-lint](https://github.com/jkaeske/django-i18n-lint)
does this with a large regex; djangofmt can do it far more precisely because it already parses
templates into an AST — text, `{{ interpolation }}`, `{% tags %}`, and comments are **distinct
node kinds**, so the regex carve-outs collapse into a few `NodeKind` matches.

This adds a new `djangofmt_lint` rule `untranslated-text` following the `add-lint-rule` skill.

### Decisions (confirmed with the maintainer)

1. **Enabled by default** (`stable_since`). djangofmt has no off-by-default mechanism today (no
   `--preview` gate, no per-rule `select`/`ignore`; `Settings::default()` = `Rule::iter().collect()`
   enables everything, and `is_preview()` is wired only into docs). A separate branch adding granular
   rule selection will land first, so this rule ships enabled and users will disable it there. **Do
   not** build preview/selection infra in this PR.
2. **Unsafe autofix for simple text only.** A standalone literal text run → wrap in
   `{% translate "…" %}` as an `Unsafe` fix. Text adjacent to `{{ interpolation }}` → diagnostic
   only (suggest `{% blocktranslate %}`, manual intervention). `FIX_AVAILABILITY = Sometimes`.
3. **Body text *and* common attributes.** Scan literal text between tags **and** the attribute
   values `alt`, `title`, `placeholder`, `aria-label`, plus `value` on
   `<input type="submit|button|reset">`.

## Key facts established during research

- `markup_fmt::ast::NodeKind` variants used here: `Text(TextNode)`, `Element`, `JinjaBlock`,
  `JinjaInterpolation`, `JinjaTag`, `Comment`, `JinjaComment`, `Doctype`. **Comments, interpolations,
  and `{% trans/translate %}` tags are NOT `Text` nodes**, so they're excluded for free.
- `TextNode { raw: &str, line_breaks: usize, start: usize }` — `start` is the byte offset of `raw`
  (no need for `source_offset`).
- Text/interp/tags are **separate sibling nodes**: `<p>Hello {{ name }}!</p>` → `Text("Hello ")`,
  `JinjaInterpolation`, `Text("!")`. So a `Text` node's `.raw` is never itself interpolated; the
  "mixed" case is detected via **sibling adjacency**.
- `NativeAttribute { name: &str, value: Option<(&str, usize)>, quote: Option<char> }` — the `usize`
  is the value's byte offset; `quote` is the delimiter char.
- `parse_jinja_tag_name(&JinjaTag) -> &str` lives in `markup_fmt::parser` (used by
  `untrimmed_blocktranslate`).
- The checker currently does **not** visit `Text` nodes (`visit_node` matches only `Element`/`JinjaBlock`).
- `<script>`/`<style>`/`<pre>`/`<textarea>` are ordinary `Element`s whose content is `Text` children
  → must be skipped.

## Architecture

> ⚠️ **See "NEEDS REFINING" at the top — the `Checker`-field approach below is a placeholder and must
> be replaced with a general ancestor-context mechanism.**

Text detection needs **sibling context** (to classify standalone vs interpolation-adjacent) and
**ancestor context** (to skip raw-text elements and `{% blocktranslate %}` bodies). Route all
children iteration through one helper and track two depth counters on the `Checker`.

### `crates/djangofmt_lint/src/checker.rs`

Add two `u32` fields (keeps `const fn new`, init both to `0`):

```rust
pub struct Checker<'a> {
    context: LintContext<'a>,
    /// Depth of enclosing raw-text elements (script/style/pre/textarea); skip text while > 0.
    raw_text_depth: u32,
    /// Depth of enclosing {% blocktranslate %}/{% blocktrans %} blocks; skip text while > 0.
    translated_depth: u32,
}
```

Add a shared children walker that does slice-aware text detection then recurses:

```rust
fn visit_children(&mut self, children: &[Node<'_>]) {
    if self.raw_text_depth == 0
        && self.translated_depth == 0
        && self.is_rule_enabled(Rule::UntranslatedText)
    {
        rules::accessibility::untranslated_text::check_text(children, self);
    }
    for child in children {
        self.visit_node(child);
    }
}
```

Wire it in (replace the three existing `for child … visit_node` loops):

- `visit_root`: `self.visit_children(&root.children);`
- `visit_element`: after the element-rule calls + attr loop, compute
  `is_raw = matches!(tag_name lowercased, "script"|"style"|"pre"|"textarea")` (use
  `eq_ignore_ascii_case`); `+= 1` / call `visit_children(&element.children)` / `-= 1`. Also add the
  new attribute check among the element rules:
  `if self.is_rule_enabled(Rule::UntranslatedText) { rules::accessibility::untranslated_text::check_attrs(element, self); }`
- `visit_jinja_block`: compute `is_translated` from the first body item's tag name
  (`parse_jinja_tag_name` → `"blocktranslate"|"blocktrans"`); `+= 1` / for each
  `JinjaTagOrChildren::Children(children)` call `visit_children(children)` / `-= 1`.

`visit_node` is unchanged (text is handled by the parent's `visit_children`, so no `Text` arm).
Depth counters persist through the recursion, so nested cases (`<pre><span>raw</span></pre>`,
`{% blocktranslate %}before <b>x</b>{% endblocktranslate %}`) are correctly skipped.

Attribute scanning intentionally ignores both depth counters (a `title=` inside `<pre>` or a
`blocktranslate` body is still untranslated — blocktranslate covers body text, not attributes).

## The rule: `crates/djangofmt_lint/src/rules/accessibility/untranslated_text.rs` (new)

Category **Accessibility** (groups with `missing-html-lang`/`missing-title`/`missing-img-alt`;
i18n is the closest existing bucket — a judgment call the maintainer can flip to `Suspicious` with a
one-line change). Lifecycle `#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]`.

### Violation struct

```rust
#[derive(Debug, PartialEq, Eq)]
enum Source { BodyStandalone, BodyInterpolated, Attribute(&'static str) }

#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct UntranslatedText { source: Source }
```

- `const RULE = Rule::UntranslatedText`, `const CATEGORY = RuleCategory::Accessibility`,
  `const FIX_AVAILABILITY = FixAvailability::Sometimes`.
- `message()` (single `#[derive_message_formats]` format string, constant):
  `"Found text that is not wrapped in a translation tag."`
- `help()` varies by `source`:
  - `BodyStandalone` / `Attribute(_)`: ``Some("Wrap the text in `{% translate %}`.".into())``
  - `BodyInterpolated`: ``Some("Wrap the surrounding text and variables in `{% blocktranslate %}…{% endblocktranslate %}`; placeholders may need manual names.".into())``
- `fix_title()`: ``Some("Wrap in `{% translate %}`".into())``

Doc comment (drives `docs/rules/untranslated-text.md`; mirror `RedundantTypeAttr` layout exactly):
`## What it does` ("Checks for literal text in templates that is not wrapped in a Django
translation tag.") → `## Why is this bad?` (untranslated literals are never collected by
`makemessages`, so they stay in the source language for every locale; follow-up paragraph listing
exclusions: script/style/pre/textarea content, `{% blocktranslate %}` bodies, text without letters,
interpolated attribute values) → `## Example`/`Use instead:` (`<p>Hello</p>` → `<p>{% translate "Hello" %}</p>`)
→ `## Fix safety` (Unsafe: requires `{% load i18n %}`, alters whitespace folded into the msgid, and
changes `.po` output — a translation-workflow decision) → `## References`
([Django i18n docs](https://docs.djangoproject.com/en/stable/topics/i18n/translation/)). Per the
skill, do **not** link django-i18n-lint in the docs (PR description only).

### Detection predicate (shared)

Flag a string iff, after trimming ASCII whitespace and stripping HTML character references
(`&name;` / `&#123;` / `&#xAB;`), it still contains ≥1 `char::is_alphabetic` (Unicode-aware — copy
is often accented/non-Latin). This drops pure number/punctuation/symbol/entity nodes
(`123.456,789`, `&nbsp;`, `· — &times;`). Add a small entity-stripping helper (in this module or
`helpers.rs`).

### `check_text(children: &[Node], checker: &Checker)` — body text

For each `(i, node)` where `node.kind` is `Text(text)` passing the predicate:
- `interpolated = is_interp(children.get(i.checked_sub(1))) || is_interp(children.get(i+1))` where
  `is_interp` matches `NodeKind::JinjaInterpolation` (guard `i == 0` with `checked_sub`).
- Span = trimmed slice: `lead = raw.len() - raw.trim_start().len(); offset = text.start + lead;`
  `(offset, trimmed.len())`.
- Report `UntranslatedText { source: if interpolated { BodyInterpolated } else { BodyStandalone } }`.
- If **not** interpolated, attach the Unsafe wrap fix:
  ```rust
  let escaped = trimmed.replace('\\', "\\\\").replace('"', "\\\"");
  guard.set_fix(Fix::unsafe_edit(Edit::replacement(
      format!("{{% translate \"{escaped}\" %}}"), (offset, trimmed.len()).into())));
  ```
  Leading/trailing whitespace stays outside the tag (`<h1>  Foo  </h1>` →
  `<h1>  {% translate "Foo" %}  </h1>`).

### `check_attrs(element: &Element, checker: &Checker)` — attribute values

Translatable set: `["alt", "title", "placeholder", "aria-label"]`, plus `"value"` when
`element.tag_name` is `input` and its `type` attr (lowercased) ∈ `{submit, button, reset}` (small
helper scanning `element.attrs`). For each `Attribute::Native(NativeAttribute { name, value: Some((value, offset)), quote })`
whose lowercased `name` is translatable:
- Skip if `contains_interpolation(value)` (helpers.rs) — `{{ }}` / `{% trans %}` already dynamic.
- Skip unless the predicate passes.
- Report `UntranslatedText { source: Attribute(static_name) }` at `(offset, value.len())`.
- Fix only when `quote` is `Some(q)` (unquoted values can't hold the spaces in the tag → report,
  no fix). Use the **opposite** inner quote so HTML attribute quoting stays valid:
  ```rust
  let inner = if q == '"' { '\'' } else { '"' };
  let escaped = value.replace('\\', "\\\\").replace(inner, &format!("\\{inner}"));
  guard.set_fix(Fix::unsafe_edit(Edit::replacement(
      format!("{{% translate {inner}{escaped}{inner} %}}"), (offset, value.len()).into())));
  // alt="A cat" -> alt="{% translate 'A cat' %}"   ;   alt='A cat' -> alt='{% translate "A cat" %}'
  ```

## Wiring (per skill)

- `rules/accessibility/mod.rs`: `pub mod untranslated_text;`
- `registry.rs`: add `(UntranslatedText, rules::accessibility::untranslated_text::UntranslatedText)`
  to `define_rules!`.
- `checker.rs`: the changes above (import `parse_jinja_tag_name`).

## Test fixtures: `crates/djangofmt_lint/tests/check/untranslated_text/`

Per the skill, **before** writing fixtures: (a) read django-i18n-lint's `tests.py` (done — table
below), (b) search its issue tracker for closed false-positive / rejected-feature reports and anchor
each as a `valid.html` regression case with a `<!-- Regression: jkaeske/django-i18n-lint #N — … -->`
cite, and (c) Sourcegraph-search real templates (`lang:HTML count:40`) for a representative handful
of real untranslated strings to ground `invalid.html`.

`untranslated_text.invalid.html` (≥1 diagnostic each; group with `<!-- … -->`): `<h1>Foo</h1>`;
`<h1>Foo</h1><p>Bar</p>`; `Foo<script>alert('Foo');</script>Bar` (only Foo/Bar flag); `Foo{{ bar }}Baz`
(two `BodyInterpolated`, no fix); `<option selected>Option</option>`; `<form method="POST">FOO</form>`;
`Foo [[yoyo]] bar` (Angular `[[ ]]` isn't Django interp); `Foo {# notrans #}` (no inline-suppress yet
→ still flags); `<title>My site</title>`; `<p>Tom &amp; Jerry</p>`; `<p>Say "hi" to the team</p>`
(quote escaping); `<h1>\n  Welcome home\n</h1>` (whitespace outside tag); `{% if cond %}Hello{% endif %}`
(ordinary block ≠ translated); attribute cases `<img alt="A cat">`, `<input type="submit" value="Confirm">`,
`<input placeholder="Search">`, `<a title='Home page'>{{ x }}</a>` (single-quoted → double-quote fix).

`untranslated_text.valid.html` (zero diagnostics — the false-positive guard): `{% trans 'Foo' %}` /
`{% translate "Foo" %}`; `{% blocktrans %}…{% endblocktrans %}`, `{% blocktranslate %}…{% endblocktranslate %}`,
with `with var=bar`, with `{% plural %}`, and one wrapping nested `<b>`; `{% load foo %}`;
`<script>alert('Foo');</script>`, `<style>.foo{content:"hi"}</style>`, `<pre>raw text</pre>`,
`<pre><span>nested raw</span></pre>`, `<textarea>default</textarea>`; `<b>123.456,789</b>`,
`<span>&nbsp;</span>`, `<span>· — &times;</span>`, whitespace-only inter-tag text; `<img src="my.jpg" ismap />`;
`<p>{{ user.name }}</p>`; `<!-- comment -->`, `{# comment #}`; interpolated attrs `<img alt="{{ photo }}">`,
`<input placeholder="{% trans 'x' %}">`; non-translatable attrs `<input type="text" value="John">`
(data value), `<a href="/">…</a>`, `<div data-id="card">…</div>`.

Per-reference-case decisions: `<h1>Foo</h1>`→invalid(+fix); `{% trans %}`/`blocktrans*`→valid;
`{% load %}`→valid; `Foo<script>…</script>Bar`→invalid(Foo/Bar); `Foo{{ bar }}Baz`→invalid(no fix);
`<option selected>Option`→invalid; `<img … ismap />`→valid; `<form>FOO</form>`→invalid;
`123.456,789`→valid; `Foo {# notrans #}`→**invalid** (no suppress mechanism — follow-up);
Angular `[[ ]]`→invalid; alt/title/value attrs→**invalid** (now in scope).

Snapshots (auto-generated, review then `cargo insta accept`): `untranslated_text.invalid.snap`
(diagnostics) and — since the fix is Unsafe — `untranslated_text.invalid.unsafe-fixed.snap` (no
plain `.fixed.snap`).

## Verification

1. `cargo test -p djangofmt_lint` → `cargo insta accept` → `cargo test -p djangofmt_lint` (green).
2. Review both snapshots: spans point at trimmed runs / attribute values; `BodyInterpolated` cases
   carry **no** fix; unsafe-fixed output is valid HTML with intact attribute quoting and no orphaned
   whitespace.
3. `just docs-generate` and commit the regenerated `docs/rules.md` + `docs/rules/untranslated-text.md`
   (CI checks these).
4. Smoke test against real templates: `~/greenday` (and optionally `just ecosystem-check-dev`) — the
   rule is on by default, so confirm it isn't catastrophically noisy/wrong on a real project; fold
   any benign systematic false positives back into `valid.html`.
5. `cargo clippy --workspace` / `cargo fmt`.

## Out of scope (note as follow-ups in the PR)

- Inline / per-file suppression (`{# notrans #}`, `{# i18n: off #}`) — djangofmt has no suppression
  framework yet.
- A `{% blocktranslate %}` autofix for interpolation-adjacent runs (multi-node span + placeholder
  synthesis + `with`/`count`/plural handling).
- `translate` vs `trans` spelling preference and any per-rule configuration (depends on the incoming
  selection/config branch).

## Commit

Single commit titled ``feat(lint): Add `untranslated-text` lint rule`` with the rule's docstring as
the body (per skill). (Global rule says no conventional-commit prefix, but the skill and this repo's
history mandate `feat(lint):` for rule additions — follow the repo convention here.)

# Plan: Tailwind stylesheet support for `unsorted-tailwind-classes`

Issue: [#525](https://github.com/UnknownPlatypus/djangofmt/issues/525), "`unsorted-tailwind-classes` does not support Tailwind plugins".

Status: **WIP plan, nothing implemented yet.**

Written on 2026-09-23 against djangofmt `25fa291f`, rustywind_core 0.7.0, Tailwind 4.3.3, daisyUI 5.7.44 and prettier-plugin-tailwindcss 0.8.1.

## TL;DR

- **The issue's "expected output" is not a sort order.** It is the reporter's original class string, unchanged. rustywind's `--output-css-file` read zero classes from their CSS, so it reordered nothing.
  - rustywind < 0.24.1 can't parse Tailwind v4's indented output.
  - No version reads anything from minified CSS.
- **djangofmt already matches prettier-plugin-tailwindcss on vanilla Tailwind v4** (9 of 10 lab cases). The gap is theme and plugin awareness.
  - rustywind_core checks colour, spacing and font-size names against Tailwind's *default* theme.
  - So `bg-base-100`, `bg-brand` and `p-gutter` are treated as unknown and move to the front.
- **We won't expose `--output-css-file`.** Instead, we add a `lint.unsorted-tailwind-classes.stylesheet` option that points at the Tailwind entry CSS.
  - djangofmt parses that file statically: `@theme`, `@utility`, `@custom-variant`, relative `@import`s and `prefix(...)`.
  - When the file contains `@plugin "daisyui"`, djangofmt applies a built-in list of daisyUI names.
  - rustywind_core gets a theme API at runtime. It lives on a fork branch pinned by git rev, and the same commits are opened as an upstream PR.

## Research findings

### What the official sorter does

This is Tailwind v4's `getClassOrder`, which prettier-plugin-tailwindcss uses. The comparator is `compareCandidates` in tailwindcss `compile.ts`. It compares, in order:

1. **Variant bitmask:** each variant's registration index, OR'd together.
2. **Property indexes:** the sorted, unique `property-order.ts` indexes of every declaration the utility emits, compared element by element. A missing element counts as ∞.
3. **Declaration count:** more declarations sort first.
4. **Class name:** a string compare that treats runs of digits as numbers.

What follows from this, all checked in the lab:

- **Theme values sort only by the CSS properties they produce.**
  - `bg-brand` emits `background-color`, just like `bg-red-500`. Ties break by name: `bg-amber-500 < bg-base-100 < bg-brand < bg-primary < bg-red-500`.
  - So a native sorter only needs to know that a key exists in a namespace. It never needs the value.
  - Exceptions, because they change the declaration count:
    - `--text-X--line-height`, `--letter-spacing` and `--font-weight`
    - `--font-X--font-feature-settings`
- **`@utility` classes and JS plugin classes sort by the declarations they emit**, including nested rules. In v4, `addComponents` is the same as `addUtilities`.
  - daisyUI `btn` has hundreds of declarations and sorts first.
  - `btn-primary` only sets `--btn-*` custom properties, so it sorts after every base utility.
  - `content-visibility` isn't in Tailwind's property order, so `content-auto` sorts last.
- **Variants sort by registration index.**
  - Core variants come first, then plugin `addVariant` variants (such as daisyUI's `is-drawer-open`), then `@custom-variant` in source order.
  - Redefining a core variant keeps its original slot.
  - Breakpoints and containers compare by their resolved value.
- **Unknown classes** have a `null` order. They go first and keep their input order.
- **Namespaces are tried in a fixed order when a root is ambiguous:**
  - `text-*`: colour, then font size.
  - `bg-*`: colour, then image.
  - `font-*`: family, then weight.
  - `shadow-*`: size, then colour.
- **v4's compiled CSS emits utilities in exactly this order.** Reading an order back from it is still lossy:
  - It only contains the classes found in scanned files.
  - Variant rules are flattened (`.hover\:x:hover`).
  - Unrelated selectors mention other classes.

### daisyUI v5

- **`@plugin "daisyui"` is a JS plugin:** `plugin.withOptions(fn, () => ({ theme: { extend: variables } }))`. Its 20 colours and 3 radius names come from `theme.extend`. They appear in no `@theme` block and in nothing a static reader can see.
- **Everything else is registered by JS calls too:** components through `addComponents`, utilities through `addUtilities`, and the `is-drawer-open` / `is-drawer-close` variants through `addVariant`.
- **The pure-CSS build has no `@theme` or `@utility`.** With `@import "daisyui/daisyui.css"`, prettier itself treats `btn` and `bg-base-100` as unknown. So the daisyUI preset must trigger only on `@plugin`.

### Why not `--output-css-file`

That flag uses rustywind_core's `Sorter::CustomSorter`. Checked in the lab with rustywind 0.28 and rustywind_core 0.7.0:

- **It replaces the pattern sorter entirely.** Unknown classes go **last**, the opposite of prettier and of djangofmt today, so turning it on re-sorts unrelated code.
- **The line regex `^\s*(\.[^\s]+)[ ]` records v4 variant rules under the wrong key.** `hover:bg-brand:hover` is stored as-is, and `.\32 xl\:block` becomes `32`.
  - The variant fallback is a hardcoded list from the v2 era, with no `aria-`, `data-`, `peer-`, `not-`, `max-` or `@container`.
  - It sorts `md:` before `hover:`.
- **Minified CSS yields zero classes, silently.**
- **CSS must be built before linting.** The result depends on how fresh the build is: pre-commit before a rebuild disagrees with CI after one. It also can't run in the wasm playground.

### How other tools do it

| Tool                                           | Mechanism                                                                                                                                                               | Plugins                                                        |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| prettier-plugin-tailwindcss                    | Loads Tailwind in Node (`tailwindStylesheet`)                                                                                                                           | Full                                                           |
| oxfmt                                          | Rust formatter that calls the prettier plugin's `createSorter` in Node                                                                                                  | Full, with reported memory use and cross-platform order issues |
| eslint-plugin-better-tailwindcss               | Tailwind JS (`entryPoint`), plus an `unknownClassPosition` option                                                                                                       | Full                                                           |
| Biome `useSortedClasses`                       | Hardcoded preset. A planned `tailwind.stylesheet` option parses `@theme`, `@utility` and `@custom-variant` natively (biomejs/biome#11396 and #11399, waiting on #11386) | Explicitly not `@plugin` or `@config`                          |
| rustywind                                      | Hardcoded v4 order, or a CSS-file / Vite / JSON order map that replaces it                                                                                              | Only through compiled CSS                                      |
| headwind, dprint-plugin-tailwindcss, leptosfmt | Hardcoded list                                                                                                                                                          | None                                                           |

**Using Tailwind's own engine isn't a good option either:**

- The engine is only exposed as `__unstable__loadDesignSystem`, and maintainers say the core API is "not considered stable for now".
- `tailwindcss canonicalize --stream` exists since 4.2.2 and ships in the standalone binary too. It rewrites classes (`w-5 h-5` becomes `size-5`) and has no sort-only mode.
- A Node-based approach would break standalone-CLI setups, which are common in Django projects (django-tailwind-cli, pytailwindcss) and have no `node_modules`.

## Decisions

| Question       | Decision                                                                                                                              |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Approach       | Parse the Tailwind entry stylesheet statically, in Rust. No Node and no build step.                                                   |
| daisyUI        | Built-in colour and radius names when `@plugin "daisyui"` is present. Components stay unknown.                                        |
| rustywind      | Theme-aware API on a branch of `UnknownPlatypus/rustywind`, pinned by git rev like markup_fmt, with the same commits opened upstream. |
| First PR scope | Full static coverage: `@theme` namespaces, `@utility`, `@custom-variant`, custom breakpoints and containers, and the prefix.          |

## Design

### Config

```toml
[tool.djangofmt.lint.unsorted-tailwind-classes]
stylesheet = "assets/css/input.css"  # relative to the pyproject.toml directory
prefix = "tw:"                        # still honoured; inferred from the stylesheet when unset
```

- **Naming:** the option mirrors prettier's `tailwindStylesheet` and Biome's `tailwind.stylesheet`.
- **pyproject only,** like the existing per-rule `prefix` (see the "per-rule config is pyproject-only" comment in `config.rs`).
- **Missing or unreadable stylesheet:** config error, exit code 2, the same as an invalid pyproject.
- **Constructs we can't read** (any `@plugin` other than daisyUI, `@config`, package `@import`s): one `debug!` or `warn!` line, never a failure.
- **Conflicting prefix:** if both `prefix` and `@import "tailwindcss" prefix(x)` are set and disagree, warn and let the explicit option win.

### Stylesheet reader (in djangofmt_lint, no IO)

- **Location:** a new module, e.g. `crates/djangofmt_lint/src/tailwind_stylesheet.rs`, or next to the rule.
- **Entry point:** `read(entry, load: impl Fn(&Path) -> io::Result<String>) -> Result<TailwindTheme, _>`. The CLI does the file reading, so the lint crate stays pure and wasm-friendly.

What it collects:

| Construct                                                      | Extracted                                                                                                                                                                                                                                             |
| -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `@theme [inline\|static\|default\|reference] { --ns-key: …; }` | A key in each namespace: `color`, `spacing`, `text`, `font`, `font-weight`, `radius`, `shadow`, `inset-shadow`, `drop-shadow`, `text-shadow`, `blur`, `leading`, `tracking`, `perspective`, `aspect`, `ease`, `animate`, `container`, `breakpoint`, … |
| `--text-hero--line-height` etc.                                | Extra declaration count for that key                                                                                                                                                                                                                  |
| `--ns-*: initial` / `--*: initial`                             | Namespace reset, which drops the default keys                                                                                                                                                                                                         |
| `--breakpoint-*`, `--container-*`                              | Name and value, used for value ordering                                                                                                                                                                                                               |
| `@utility name { … }` / `@utility name-* { … }`                | Name or root, plus the property names of all declarations (nested rules included) and the custom-property count. `--value()` is not resolved, so any value is accepted                                                                                |
| `@custom-variant name …`                                       | Name, in source order                                                                                                                                                                                                                                 |
| `@import "./x.css"` / `"../x.css"`                             | Followed recursively, with a cycle guard. Bare specifiers are skipped                                                                                                                                                                                 |
| `@import "tailwindcss" prefix(tw)`                             | The prefix                                                                                                                                                                                                                                            |
| `@plugin "daisyui"` / `"./daisyui.mjs"`                        | Turns on the daisyUI preset                                                                                                                                                                                                                           |

**Parser:** prefer `raffia`, which is already in the dependency tree through malva (0.12.3). It must round-trip Tailwind's at-rules: `@theme`, `@utility x-*`, `@custom-variant x (…);`, `--value(…)` and `@slot`. If it can't, write a small scanner for at-rules and declarations. Spike this first.

### daisyUI preset

- **Colours (20):** `base-100 base-200 base-300 base-content primary primary-content secondary secondary-content accent accent-content neutral neutral-content info info-content success success-content warning warning-content error error-content`.
- **Radius (3):** `selector field box`, used as `rounded-box` etc.
- **To check while implementing:**
  - Compare the list against `daisyui/functions/variables.js`, and add a test that pins it.
  - Find out how daisyUI's own `prefix` plugin option interacts with these names. Does it prefix colours too?

### rustywind_core changes (fork branch, then upstream PR)

**Base:** upstream `avencera/rustywind` at the rustywind_core 0.7.0 release (rustywind v0.28.0). Work in `~/workspace/rustywind` and add an `upstream` remote, because the fork's `master` is stale.

Proposed API (names to settle with upstream):

```rust
pub struct Theme {
    keys: HashMap<Namespace, HashSet<String>>, // extra theme keys
    resets: HashSet<Namespace>,                // `--ns-*: initial`
    extra_declarations: HashMap<(Namespace, String), usize>,
    breakpoints: Vec<(String, String)>,        // name, CSS value
    containers: Vec<(String, String)>,
    utilities: Vec<CustomUtility>,             // exact name or functional root, properties, declaration count
    variants: Vec<String>,                     // custom variants in source order
}

HybridSorter::with_theme(prefix, Arc<Theme>)
RustyWind { theme: Option<Arc<Theme>>, .. } // or a `Sorter` variant carrying the themed sorter
```

Where it plugs in:

- **`utility_map.rs`:** `match_pattern` checks values with `is_color_value` (about 47 call sites), `is_spacing_value`, `is_size_keyword`, `is_weight_keyword`, the shadow-size helpers and so on.
  - Thread `&Theme` through and OR the theme keys in, after stripping `/opacity` modifiers.
  - A namespace reset turns the default check off for that namespace.
  - Keep Tailwind's precedence for ambiguous roots (for `text-*`: colour before font size, etc.).
- **`PatternSorter::get_sort_key`:** resolve custom utilities before `UTILITY_MAP`, sending their properties through `property_order::get_property_index`.
  - A known utility with no ordered property must sort after the base utilities (Tailwind's ∞), not become `None`.
  - Today `property_indices.is_empty()` returns `None`.
- **`variant_order.rs`:** `VARIANT_ORDER` has 84 entries. The bitmask uses bits below 120, and bit 120 marks arbitrary variants.
  - Custom variants take indices from 84 up, which leaves 36 slots. Overflow falls back to the arbitrary bucket; document that.
  - A custom variant that redefines a built-in name keeps the built-in index.
- **Breakpoints and containers:** breakpoints (`sm`…`2xl`, `max-*`, `min-*`) and containers (`@sm`…) currently have fixed slots.
  - Switch them to value comparison, using the default values unless the theme overrides them.
  - Container queries already have a value comparison to build on: `compare_container_breakpoint_values`.
- **Global caches:** `PATTERN_SORTER` and `PREFIXED_PATTERN_SORTERS` are keyed by prefix only. A themed sorter should be built once per run and owned by the caller.

### djangofmt wiring

- **`pyproject.rs`:** add `UnsortedTailwindClassesOptions.stylesheet: Option<PathBuf>`, documented with `#[option(...)]`.
- **`config.rs` / `commands/check.rs`:** resolve the path against `project_root`. Read and parse it once in `CheckRun::new` (the stdin path too), then build the themed sorter.
- **`settings.rs`:** `unsorted_tailwind_classes::Settings { prefix, theme: Option<Arc<…>> }`.
  - `Arc`, because `settings_for` clones `Settings` for each file that matches a per-file ignore.
  - Keep the `PartialEq`/`Eq` derive working: either compare with `Arc::ptr_eq`, or store the parsed theme, which is `Eq`.
- **`unsorted_tailwind_classes.rs`:** pass the themed sorter to `RustyWind`.
  - The rule doc's `## Options` gains `lint.unsorted-tailwind-classes.stylesheet`.
  - Add a short section on what is and isn't understood: daisyUI components and other JS plugins stay unknown.
- **Playground (wasm):** unchanged, no stylesheet.
- **`Cargo.toml`:** pin `rustywind_core` to the fork rev, keeping `default-features = false`.

## Expected results

Lab cases, with Tailwind 4.3.3, daisyUI 5.7.44 and prettier-plugin-tailwindcss 0.8.1 using `tailwindStylesheet`. The prettier and "today" columns were run. The "after" column is what this plan should produce; ✅ means it matches prettier.

| Input                                                                         | prettier (target)                                                             | djangofmt today                                                               | After this plan                                                                |
| ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `bg-base-100 flex h-5 w-5 gap-px rounded border border-current/15 p-0.5`      | `flex h-5 w-5 gap-px rounded border border-current/15 bg-base-100 p-0.5`      | `bg-base-100 flex h-5 w-5 …`                                                  | ✅                                                                             |
| `md:bg-primary/50 text-base-content border-base-300`                          | `border-base-300 text-base-content md:bg-primary/50`                          | unchanged                                                                     | ✅                                                                             |
| `p-gutter text-hero text-brand bg-brand/20 mt-2`                              | `mt-2 bg-brand/20 p-gutter text-hero text-brand`                              | unchanged                                                                     | ✅                                                                             |
| `content-auto pointer-fine:underline hocus:underline flex`                    | `flex content-auto pointer-fine:underline hocus:underline`                    | `flex content-auto hocus:underline pointer-fine:underline`                    | ✅ (needs `pointer-fine` in rustywind's built-in variant list if it's missing) |
| `data-[state=open]:bg-base-100 group-hover:text-primary [&>svg]:size-4 *:p-1` | `*:p-1 group-hover:text-primary data-[state=open]:bg-base-100 [&>svg]:size-4` | `data-[state=open]:bg-base-100 group-hover:text-primary *:p-1 [&>svg]:size-4` | ✅ (inferred)                                                                  |
| `my-custom-class js-toggle flex bg-reed-500 p-2`                              | `my-custom-class js-toggle bg-reed-500 flex p-2`                              | same as prettier                                                              | ✅                                                                             |
| `btn btn-primary btn-sm p-4 flex`                                             | `btn flex p-4 btn-primary btn-sm`                                             | `btn btn-primary btn-sm flex p-4`                                             | ❌ same as today: components stay unknown                                      |
| `card card-body shadow-sm bg-base-200`                                        | `card-body card bg-base-200 shadow-sm`                                        | `card card-body bg-base-200 shadow-sm`                                        | ❌ same as today                                                               |
| `hover:btn-active md:btn-lg text-primary-content`                             | `text-primary-content hover:btn-active md:btn-lg`                             | unchanged                                                                     | ❌ same as today                                                               |
| `rounded-box badge badge-outline sm:card-body is-drawer-open:hidden`          | `badge rounded-box badge-outline sm:card-body is-drawer-open:hidden`          | unchanged                                                                     | ❌ `badge badge-outline sm:card-body is-drawer-open:hidden rounded-box`        |

With daisyUI loaded, prettier agrees with djangofmt on 1 of 10 cases today, and should agree on 6 of 10 after this plan. The remaining misses are all daisyUI component classes.

## Work breakdown

### rustywind

Branch `theme-aware-sorting` on UnknownPlatypus/rustywind, from upstream v0.28.0.

1. Thread an empty `Theme` through `PatternSorter` and `UtilityMap`, with no behaviour change. The parity suite must stay green.
2. Theme keys, namespace resets, and sub-keys that add declarations.
3. Custom utilities, exact and functional, including the "known but unordered" placement.
4. Custom variants, appended after the built-ins.
5. Custom breakpoints and containers, ordered by value.
6. A public constructor on `RustyWind` / `HybridSorter` that takes the theme.

Each step comes with unit tests whose expected output comes from prettier-plugin-tailwindcss. rustywind already has a parity harness; extend it with a stylesheet fixture. Then open the same commits upstream.

### djangofmt

A branch off `main`. Only the final commit gets the `feat(lint): …` prefix.

1. Pin rustywind_core to the fork rev.
2. Stylesheet reader, with one unit test per construct.
3. daisyUI preset.
4. `stylesheet` option: pyproject, path resolution against the project root, settings, rule wiring and error handling.
5. Docs: rule options plus a limitations section; check the output with `just docs-generate`.
6. CLI integration test in a tempdir containing a pyproject, a stylesheet with `@theme` and `@plugin "daisyui"`, and a template, run with `check --select unsorted-tailwind-classes --fix`.

## Testing and validation

- Run `just pre-mr-check`.
- Re-run the lab (see the appendix) and compare against prettier, using the expected results table above.
- Run the `djangofmt_benchmark` benchmarks: the path without a stylesheet must not regress. Themed lookups should only happen when a theme is set.
- Keep tests lean (see CLAUDE.md): one parser unit test per construct and one end-to-end CLI test. Don't re-test rustywind's ordering from djangofmt.

## Out of scope and follow-ups

- **daisyUI components** (`btn`, `card`, `btn-primary`): would need a class → properties table regenerated for each daisyUI release.
- **daisyUI's `is-drawer-open` / `is-drawer-close` variants:** two names, cheap to add to the preset if wanted.
- **Other JS plugins,** `@config` (v3 JS config), and Tailwind v3 projects.
- **`--value()` resolution** for functional `@utility`; we accept any value.
- **Stylesheet support in the playground.**
- **An `unknown-class-position` option** (first / last), as in headwind and the ESLint plugins. Not requested.
- **An exact mode that runs Tailwind itself,** through the Node design system or a future sort-only `tailwindcss canonicalize`.

## Risks and open questions

- **Upstream acceptance:** rustywind PR #150 (heuristic named colours) was closed without explanation. An explicit theme API is a different kind of change, but we may carry the fork for a while.
- **raffia's handling of Tailwind at-rules:** needs the spike.
- **Precedence for ambiguous roots** (`text-`, `font-`, `shadow-`, `bg-`) once custom keys are added.
- **Semantics of daisyUI's `prefix` option.**
- **Variant slot overflow** beyond 36 custom variants.
- **The reporter's setup** (rustywind version, minified CSS?): ask on the issue.

## Draft reply for #525

> Thanks for the report! I dug into this and found something surprising: the "expected output" in the issue is actually your original class order, left untouched. With rustywind older than 0.24.1 (it can't read Tailwind v4's indented CSS), or with a minified CSS file, `--output-css-file` extracts zero classes, so it reorders nothing. With rustywind 0.28 on an unminified build, it gives `flex h-5 w-5 gap-px rounded border border-current/15 bg-base-100 p-0.5`. That's also what prettier-plugin-tailwindcss produces with daisyUI loaded.
>
> Rather than exposing the CSS-file mode, the plan is a `stylesheet` option that points at your Tailwind entry CSS. The CSS-file mode moves unknown classes last, sorts most v4 variants in the wrong order, and depends on the CSS being rebuilt before linting. With the new option, djangofmt will read `@theme`, `@utility` and `@custom-variant`, and recognise `@plugin "daisyui"`, so `bg-base-100` and friends sort like any other colour. Could you tell me which rustywind version you used, and whether your CSS was minified?

## Appendix: reproducing the lab

```jsonc
// package.json devDependencies
"@tailwindcss/cli": "^4.3.3", "@tailwindcss/node": "^4.3.3", "tailwindcss": "^4.3.3",
"daisyui": "^5.7.44", "prettier": "^3.9.9", "prettier-plugin-tailwindcss": "^0.8.1", "rustywind": "^0.28.0"
```

```css
/* css/app.css */
@import 'tailwindcss';
@plugin 'daisyui';
@theme {
    --color-brand: #123456;
    --spacing-gutter: 3rem;
    --text-hero: 4rem;
}
@utility content-auto {
    content-visibility: auto;
}
@custom-variant pointer-fine (@media (pointer: fine));
@custom-variant hocus (&:hover, &:focus);
```

```jsonc
// prettier/app.json
{
  "plugins": ["prettier-plugin-tailwindcss"],
  "tailwindStylesheet": "./css/app.css",
  "printWidth": 1000
}
```

- Wrap each line of the cases (the inputs in the table above) as `<div class="…"></div>` in `html/cases.html`.
- **prettier:** `npx prettier --config prettier/app.json html/cases.html`
- **djangofmt:** `djangofmt check --select unsorted-tailwind-classes --fix cases.html`
- **Compiled CSS:** `npx @tailwindcss/cli -i css/app.css -o build/cases.css`, plus `--minify` for the minified variant. The entry needs an `@source` covering `html/`.
- **rustywind CSS mode:** `npx rustywind --stdin --output-css-file build/cases.css < html/cases.html`
- **Design system:** `__unstable__loadDesignSystem(css, { base })` from `@tailwindcss/node`, then `getClassOrder([...])` / `getClassList()` / `getVariants()`. Loading takes about 66 ms with daisyUI, and daisyUI prints a banner to stdout.

# Benchmarks

Documentation about how to run various performance benchmark for `djangofmt` versus similar tools:

- `djlint`: Same scope as `djangofmt` - fully featured template formatter
- `djhtml`: Only an indenter, it will never add/remove newlines
- `djade`: Only format django template syntax - HTML is not formatted
- `prettier`: Does not support Django and only format HTML

## Running Benchmarks

Simply run this command, providing a directory containing django templates.
You can change the print width with the `LINE_LENGTH` env variable (default: 120)

```bash
just bench-py ~/templates
```

A setup step will discover every html files inside and then run the various tools on it.

> [!IMPORTANT]
> This will cause destructive operations, be sure to target a safe directory (tracked with git or temporary)

## Regenerating the chart

The bar chart in [`docs/benchmarks.md`](../../docs/benchmarks.md) is a Vega-Lite spec
rendered to SVG. [`chart.vl.json`](./chart.vl.json) is the source of truth: update the
`data.values` entries with the new hyperfine means, then render both color schemes with

```bash
just bench-chart
```

This writes `benchmark-light.svg` and `benchmark-dark.svg`, which differ only by the
`labelColor` config param. Both are gitignored: GitHub hosts them, not the repo.

To publish them, drag both files into any GitHub comment box (an issue, a PR, or a
draft you never submit). GitHub uploads them and rewrites the markdown to
`https://github.com/user-attachments/assets/<uuid>` — permanent URLs that survive
discarding the comment. Paste those two URLs into the `<picture>` block of both
`README.md` and `docs/benchmarks.md` (dark first, light as both the second `<source>`
and the fallback `<img>`).

Color palette:

- bars -> `#187f58`
- labels (light mode) -> `#333333`
- labels (dark mode) -> `#c9d1d9`

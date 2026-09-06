# Lint rules

By default djangofmt runs every stable rule.

Override that with `select` and `ignore`, either on the command line (`--select`, `--ignore`) or under `[tool.djangofmt.lint]` in `pyproject.toml`.

```toml
[tool.djangofmt.lint]
select = ["category:all"]
ignore = ["category:style", "missing-img-alt"]
preview = true
```

A selector is either:

- single rule name (e.g. `missing-img-alt`)
- a group prefixed with `category:` (e.g. `category:all`, `category:style`, ...)

Preview rules are off by default. Enable them with `--preview` or `preview = true`.

To turn rules off for some files only, map a glob to the selectors to ignore there:

```toml
[tool.djangofmt.lint.per-file-ignores]
"legacy/*" = ["category:accessibility"]
"emails/*.html" = ["missing-img-dimensions", "use-https"]
```

Every pattern is matched both against the file name and against the path relative to the directory holding `pyproject.toml`: `"*.jinja"` covers that extension at any depth, while `"emails/*.html"` is anchored at the project root (and, since `*` crosses `/`, covers nested files under `emails/` too). These are ruff's `per-file-ignores` semantics, so a block can be copied over unchanged.

## Suppressing diagnostics

A `{# djangofmt: ignore[...] #}` comment silences the listed rules on the next node. Whitespace and other comments between the directive and its target are skipped, so directives can stack.

```jinja
{# djangofmt: ignore[invalid-attr-value, empty-attr-value] #}
<form method="yes" id=""></form>
```

A `{# djangofmt: file-ignore[...] #}` comment at the **very top of the file**, before any markup, covers the whole file. It also accepts two codes that are not rules:

- `invalid-syntax` skips a file neither command can parse
- `format` skips the formatter (see [Disabling formatting](formatting.md#disabling-formatting) for more details)

```jinja
{# djangofmt: file-ignore[invalid-syntax] #}
```

A directive djangofmt cannot honor is reported rather than skipped silently:
- [`invalid-ignore-comment`](rules/invalid-ignore-comment.md) for a malformed or misplaced one
- [`invalid-ignore-code`](rules/invalid-ignore-code.md) for a code naming no rule
- [`unused-ignore-code`](rules/unused-ignore-code.md) for a code that silences nothing.

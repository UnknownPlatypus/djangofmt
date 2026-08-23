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

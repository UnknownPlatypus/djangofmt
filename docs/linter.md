# Linting

`djangofmt check` runs the [lint rules](rules.md) over your templates and reports every violation it finds:

```shell
djangofmt check .        # Report violations
djangofmt check --fix .  # Apply safe fixes, then report what is left
```

Files are discovered the same way as for formatting, with the same [file selection](settings.md#exclude) options.

## Rule selection

By default djangofmt runs every stable rule outside the `pedantic` category.
Override that with `select` and `ignore`, either on the command line (`--select`, `--ignore`) or under `[tool.djangofmt.lint]` in `pyproject.toml`:

```toml
[tool.djangofmt.lint]
select = ["category:default", "unsorted-tailwind-classes"]
ignore = ["category:style", "missing-img-alt"]
```

A selector is either a rule name (`missing-img-alt`) or a group prefixed with `category:` (`category:default`, `category:all`, `category:style`, ...).
A more specific selector always wins, regardless of order.

Rules marked 🧪 in the [rules table](rules.md) are in preview: they stay off until enabled with `--preview` or `preview = true`.

To turn rules off for some files only, map a glob to the selectors to ignore there:

```toml
[tool.djangofmt.lint.per-file-ignores]
"legacy/*" = ["category:accessibility"]
"emails/*.html" = ["missing-img-dimensions", "use-https"]
```

Every pattern is matched both against the file name and against the path relative to the directory holding `pyproject.toml`: `"*.jinja"` covers that extension at any depth, while `"emails/*.html"` is anchored at the project root (and, since `*` crosses `/`, covers nested files under `emails/` too). These are ruff's `per-file-ignores` semantics, so a block can be copied over unchanged.

## Fixes

Rules marked 🛠️ in the [rules table](rules.md) can fix what they report. Fixable violations are marked `[*]` in the output, and `--fix` applies them.

A fix is *safe* when it cannot change what the template renders, and *unsafe* when it might (rewriting `http://` to `https://`, for instance).
`--fix` only applies safe fixes; the summary counts the unsafe ones as hidden. Add `--unsafe-fixes` to apply them too, and review the diff.
A rule whose fix is unsafe says why under "Fix safety" on its page.

`--show-fixes` lists how many fixes each rule applied.

A fix can leave the template needing a reformat, so run the formatter after `check --fix`.

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
- [`unused-ignore-code`](rules/unused-ignore-code.md) for a code that silences nothing

## Exit codes

`djangofmt check` exits with the following status codes:

- `0` if no violation remains after the automatic fixes.
- `1` if violations were found.
- `2` if djangofmt terminates abnormally: a file that does not parse, an I/O error, invalid configuration or invalid CLI options.

`djangofmt --check` follows the same convention, exiting with `1` when a file would be reformatted.
This mirrors tools like Ruff, ESLint and Prettier.

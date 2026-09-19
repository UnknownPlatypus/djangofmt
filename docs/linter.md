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

## Fixes

Rules marked 🛠️ in the [rules table](rules.md) can fix what they report. Fixable violations are marked `[*]` in the output, and `--fix` applies them.

A fix is *safe* when it cannot change what the template renders, and *unsafe* when it might. Add `--unsafe-fixes` to apply them too, and review the diff.
A rule whose fix is unsafe says why under "Fix safety" on its documentation page.

## Suppressing diagnostics

A `{# djangofmt: ignore[...] #}` comment silences the listed rules on the next node.

```jinja
{# djangofmt: ignore[invalid-attr-value, empty-attr-value] #}
<form method="yes" id=""></form>
```

A `{# djangofmt: file-ignore[...] #}` comment at the **very top of the file**, before any markup, covers the whole file. It also accepts two codes that are not rules:

- `invalid-syntax` skips a file failing to parse
- `format` skips the formatter (see [Disabling formatting](formatting.md#disabling-formatting) for more details)

```jinja
{# djangofmt: file-ignore[invalid-syntax] #}
```

The linter contains a few meta-rules to ensure proper usage of these ignore directives:

- [`invalid-ignore-comment`](rules/invalid-ignore-comment.md) for a malformed or misplaced one
- [`invalid-ignore-code`](rules/invalid-ignore-code.md) for a code naming no rule
- [`unused-ignore-code`](rules/unused-ignore-code.md) for a code that silences nothing

## Exit codes

`djangofmt check` exits with the following status codes:

- `0` if no violation remains after the automatic fixes.
- `1` if violations were found.
- `2` if djangofmt terminates abnormally: a file that does not parse, an I/O error, invalid configuration or invalid CLI options.

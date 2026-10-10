# Coming from djade

[djade](https://github.com/adamchainz/djade) formats Django templates following the [Django template style guide](https://docs.djangoproject.com/en/stable/internals/contributing/writing-code/coding-style/#template-style), and upgrades deprecated syntax for a `--target-version`.
djangofmt covers most of it: whitespace and layout belong to the formatter, and every rewrite of the template syntax is a lint rule with a safe fix.

Run `djangofmt check --fix` before `djangofmt`, as in the [pre-commit sample](index.md#pre-commit-hook).
The label rules read the layout at fix time, so their fixes can lag one run behind the formatter.
djade's `--check`, stdin via `-` and pre-commit hook map to `djangofmt --check`, `--stdin-filename` and the [pre-commit hook](index.md#pre-commit-hook).

## Formatting

djangofmt does all of djade's formatting under the `django` profile, except the `{% load %}` rewrites (merging consecutive tags, sorting the libraries and the imported items), see [known limitations](known-limitations.md#load-tags-are-left-as-written).
Inside attribute values other than `style`, tags and variables are printed as written, see [known limitations](known-limitations.md#template-tags-inside-text).

| djade                                                                     | Differences                                                                                                                                                         |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Single spaces at the start and end of variables and tags                  |                                                                                                                                                                     |
| Single spaces between the tokens of a tag                                 |                                                                                                                                                                     |
| No spaces around filters in variables                                     | A variable Django cannot compile is left as written                                                                                                                 |
| Single spaces at the start and end of comments                            | A comment whose text starts or ends with `-`, such as a Jinja-style `{#- … -#}`, is left as written                                                                 |
| No leading or trailing empty lines                                        |                                                                                                                                                                     |
| Unindent `{% extends %}` and the top-level `{% block %}` tags             | Every top-level tag starts its line, with or without `{% extends %}`                                                                                                |
| One blank line between top-level `{% block %}` tags under `{% extends %}` | Also one blank line after `{% extends %}` and on both sides of every top-level block, whatever sits next to it. Comments directly above a block stay attached to it |
| Keeps CRLF line endings                                                   | djangofmt always writes LF, see [known limitations](known-limitations.md#output-is-always-lf)                                                                       |

## Labels and fixers

Gate is the Django version [`target-version`](settings.md#lint_target-version) must reach, as in djade.
No rule reads a `{% comment %}` body, which Django never compiles; djade renames inside it.

| djade                                                  | djangofmt                                                             | Gate | Differences                                                                                                                                                                                                        |
| ------------------------------------------------------ | --------------------------------------------------------------------- | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Label `{% endblock %}` and `{% endpartialdef %}` tags  | [`missing-endblock-label`](rules/missing-endblock-label.md)           |      |                                                                                                                                                                                                                    |
| No labels on a single-line block                       | [`redundant-endblock-label`](rules/redundant-endblock-label.md)       |      | Only removes a label that repeats the block's name, where djade also strips a mismatched one                                                                                                                       |
| `length_is` → `length`                                 | [`deprecated-length-is-filter`](rules/deprecated-length-is-filter.md) | 4.2  | Only in `{% if %}` and `{% elif %}`, where djade rewrites any two-token tag; the filters before `length_is` are kept, where djade drops them                                                                       |
| Empty ID `json_script`                                 | [`redundant-json-script-id`](rules/redundant-json-script-id.md)       | 4.1  |                                                                                                                                                                                                                    |
| `trans` → `translate`, `blocktrans` → `blocktranslate` | [`legacy-translation-tag`](rules/legacy-translation-tag.md)           | 3.1  | A `{% load trans from i18n %}` and the tags using the name get separate fixes, so ignoring one of those lines but not the others leaves a template Django rejects                                                  |
| `ifequal` and `ifnotequal` → `if`                      | [`deprecated-ifequal-tag`](rules/deprecated-ifequal-tag.md)           | 3.1  | A closing tag must match its opener, and an operand that is an `if` operator, such as `in`, is left alone; a pair inside text is not seen, see [known limitations](known-limitations.md#template-tags-inside-text) |
| `admin_static` and `staticfiles` → `static`            | [`deprecated-static-library`](rules/deprecated-static-library.md)     | 2.1  |                                                                                                                                                                                                                    |
| Legacy variable assignment syntax                      | [`legacy-as-assignment`](rules/legacy-as-assignment.md)               |      | `{% with a=b as c %}` is left alone, where djade writes `c=a=b`, and `count n` is never dropped                                                                                                                    |

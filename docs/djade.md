# Coming from djade

[djade](https://github.com/adamchainz/djade) formats Django templates following the [Django template style guide](https://docs.djangoproject.com/en/stable/internals/contributing/writing-code/coding-style/#template-style), and upgrades deprecated syntax for a `--target-version`.
djangofmt covers most of it: whitespace and layout belong to the formatter, and every rewrite of the template syntax is a lint rule with a safe fix, which you can turn off like any other rule.

Run the formatter before the fixes, as in the [pre-commit sample](index.md#pre-commit-hook):

```shell
djangofmt .
djangofmt check --fix .
```

The label rules decide from the line layout, which the formatter may change: in the other order, a second run can still find labels to fix.
After the first run on a project, format once more: a renamed tag such as `{% translate %}` is longer, and can push a line past the line length.

## Formatting

djangofmt does all of djade's formatting under the `django` profile, except the `{% load %}` rewrites.

| djade                                                                     | djangofmt | Differences                                                                                                                                                         |
| ------------------------------------------------------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Single spaces at the start and end of variables and tags                  | Formatter |                                                                                                                                                                     |
| Single spaces between the tokens of a tag                                 | Formatter | Not inside attribute values, see [known limitations](known-limitations.md#template-tags-inside-attribute-values)                                                    |
| No spaces around filters in variables                                     | Formatter | A variable Django cannot compile is left as written. Not inside attribute values other than `style`                                                                 |
| Single spaces at the start and end of comments                            | Formatter | `{#- … -#}` becomes `{# - … - #}`, see [known limitations](known-limitations.md#whitespace-control-in-comments)                                                     |
| No leading or trailing empty lines                                        | Formatter |                                                                                                                                                                     |
| Unindent `{% extends %}` and the top-level `{% block %}` tags             | Formatter | Every top-level tag starts its line, with or without `{% extends %}`                                                                                                |
| One blank line between top-level `{% block %}` tags under `{% extends %}` | Formatter | Also one blank line after `{% extends %}` and on both sides of every top-level block, whatever sits next to it. Comments directly above a block stay attached to it |
| Merge consecutive `{% load %}` tags                                       | Not done  | One library per `{% load %}` line is the preferred style, see [known limitations](known-limitations.md#load-tags-are-left-as-written)                               |
| Sort libraries in `{% load %}` tags                                       | Not done  | Reordering libraries can change which tag wins                                                                                                                      |
| Sort loaded items in `{% load … from … %}` tags                           | Not done  |                                                                                                                                                                     |

## Labels and fixers

These are lint rules, all in the default selection and fixed by `djangofmt check --fix`.
A rule with a gate reports nothing until [`target-version`](settings.md#lint_target-version), set or inferred from `[project] dependencies`, names that Django version or a newer one; djade reads it from `--target-version`.

| djade                                                  | djangofmt                                                             | Gate | Differences                                                                                                                                                                               |
| ------------------------------------------------------ | --------------------------------------------------------------------- | ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Label `{% endblock %}` and `{% endpartialdef %}` tags  | [`missing-endblock-label`](rules/missing-endblock-label.md)           |      | A label that names another block is left alone, since Django rejects it                                                                                                                   |
| No labels on a single-line block                       | [`redundant-endblock-label`](rules/redundant-endblock-label.md)       |      | Only removes a label that repeats the block's name, where djade also strips a mismatched one                                                                                              |
| `length_is` → `length`                                 | [`deprecated-length-is-filter`](rules/deprecated-length-is-filter.md) | 4.2  | Only in `{% if %}` and `{% elif %}`, where djade rewrites any tag of two tokens. The filters before `length_is` are kept, and a chain that goes on after it is left alone                 |
| Empty ID `json_script`                                 | [`redundant-json-script-id`](rules/redundant-json-script-id.md)       | 4.1  |                                                                                                                                                                                           |
| `trans` → `translate`, `blocktrans` → `blocktranslate` | [`legacy-translation-tag`](rules/legacy-translation-tag.md)           | 3.1  | `{% comment %}` bodies are left alone, since Django never compiles them                                                                                                                   |
| `ifequal` and `ifnotequal` → `if`                      | [`deprecated-ifequal-tag`](rules/deprecated-ifequal-tag.md)           | 3.1  | Pairs come from the parsed template, so a closing tag must match its opener. An operand that is an `if` operator, such as `in`, is left alone, and so is a pair inside an attribute value |
| `admin_static` and `staticfiles` → `static`            | [`deprecated-static-library`](rules/deprecated-static-library.md)     | 2.1  | `{% load staticfiles static %}` becomes `{% load static %}`, not `{% load static static %}`                                                                                               |
| Legacy variable assignment syntax                      | [`legacy-as-assignment`](rules/legacy-as-assignment.md)               |      | Only rewrites assignments Django accepts, plus a missing `and` between two of them: `{% with a=b as c %}` is left alone, where djade writes `c=a=b`                                       |

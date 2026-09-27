# Controlling the formatting

Djangofmt gives users control over formatting in cases where
static analysis struggles to determine the optimal approach.

## Splitting an opening tag across multiple lines

You can control this formatting by choosing whether to insert a newline before the first attribute:

```diff
# Unchanged
<div class="flex" id="great" data-a>
  This is nice!
</div>

# Wrap on multiple lines
<div
-    class="flex" id="great" data-a>
+    class="flex"
+    id="great"
+    data-a
+>
    This is nice!
</div>
```

## Class attribute formatting

The `class` attribute will be formatted as a space-separated sequence of strings,
unless there are already newlines inside the attribute value.

This makes it possible to accommodate the 2 following use cases:

```html
<div class="
  mt-8 p-8
  bg-indigo-600 hover:bg-indigo-700
  border border-transparent
  font-medium text-white
">
    Hello world
</div>

<div class="mt-8 p-8 bg-indigo-600 hover:bg-indigo-700 border border-transparent font-medium text-white">
    Hello world
</div>
```

See https://github.com/g-plane/markup_fmt/issues/75#issuecomment-2456526352 for the rationale.

## Keeping element content as written

List elements whose content matters byte for byte, such as a markdown component, in `raw-elements`:

```toml
[tool.djangofmt]
raw-elements = ["c-markdown", "c-code-block"]
```

Their content is kept exactly like `<pre>`: it is read as raw text, so unbalanced HTML inside is fine,
while the opening tag's attributes are still formatted.
The element closes at the first matching end tag, so nesting the same element isn't supported.

For a one-off element, prefer a [`{# djangofmt: ignore[format] #}`](#disabling-formatting) comment.

## Disabling formatting

To disable formatting for an entire file, add `{# djangofmt: file-ignore[format] #}` at the very top of the file:

```html
{# djangofmt: file-ignore[format] #}
<div   class="keep-this-unformatted"   >Content</div>
```

To disable formatting for a specific node, prefix it with a `{# djangofmt: ignore[format] #}` comment:

```html
{# djangofmt: ignore[format] #}
<div   class="keep-this-unformatted"   >Content</div>
<div class="this-will-be-formatted">Content</div>
```

A file that djangofmt cannot parse at all can be skipped with `{# djangofmt: file-ignore[invalid-syntax] #}`
at the very top: both `format` and `check` skip it instead of reporting a parse error.

Anything after the closing bracket is a free-text reason:

```html
{# djangofmt: file-ignore[format]: generated file, do not touch #}
```

For backward compatibility reasons, a bare `{# djangofmt:ignore #}` (or `<!-- djangofmt:ignore -->`) still disables formatting.
You can use the [`deprecated-ignore`](rules/deprecated-ignore.md) rule to automatically rewrites both spellings to `ignore[format]`.

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

## Preserving unquoted attribute values

By default, djangofmt quotes all attribute values:

```diff
- <c-button editable=True count=42 />
+ <c-button editable="True" count="42" />
```

Enable `preserve-unquoted-attrs` to suppress this transformation and keep them unquoted.
This is useful for frameworks like [Django Cotton](https://django-cotton.com/) that use unquoted
attribute values to pass non-string types (booleans, numbers, template variables).

## Disabling formatting

To disable formatting for an entire file, add `<!-- djangofmt:ignore -->` at the very top of the file.

To disable formatting for a specific node, prefix it with the same comment:

```html
<!-- djangofmt:ignore -->
<div   class="keep-this-unformatted"   >Content</div>
<div class="this-will-be-formatted">Content</div>
```

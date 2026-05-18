# Known limitations

## `style` attributes formatting

The `style` attribute will be formatted using a CSS formatter ([Malva](https://github.com/g-plane/malva)),
but the output will always be on a single line.

**Before:**

```html
<div class="flex flex-col items-center absolute z-10"
     style="top:60%;
            transform:translate(0,-50%)">
    Such a lovely day
</div>
```

**After:**

```html
<div class="flex flex-col items-center absolute z-10"
     style="top:60%; transform:translate(0,-50%)">
    Such a lovely day
</div>
```

## Conditional open/close tags

Djangofmt doesn't accept and will produce parsing errors for any syntax that could cut off HTML in obvious ways, e.g.:

```html
{% if condition %}
    <div class="container">
{% endif %}
    Some content
{% if condition %}
    </div>
{% endif %}
```

This is generally discouraged and should be avoided because it's an easy way to create invalid HTML.

You can almost always write it another way that is much more readable. For example:

```diff
-<div {{ attr_name }}{% if not boolean_attr %}="{{ attr_value }}"{% endif %}></div>
+<div
+    {% if boolean_attr %}
+        {{ attr_name }}
+    {% else %}
+        {{ attr_name }}="{{ attr_value }}"
+    {% endif %}
+></div>
```

See upstream tracking issue: https://github.com/g-plane/markup_fmt/issues/97

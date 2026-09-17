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

## Omitted end tags

HTML5 lets you leave out the end tag of a handful of elements (`</li>`, `</p>`, `</tr>`, `</td>`, `</dt>`, `</dd>`, `</option>`, `</body>`, `</html>`, ...) when the parser can infer it from what follows.
Djangofmt requires them and reports a parse error instead:

```html
<ul>
    <li>Coffee
    <li>Tea
</ul>
```

```
× expected close tag for opening tag <li>
```

Write the close tags explicitly:

```html
<ul>
    <li>Coffee</li>
    <li>Tea</li>
</ul>
```

The inferred form is legal, but it depends on rules few people know by heart, and interleaving template tags with it makes the intended structure ambiguous.
Requiring the close tag keeps the document unambiguous for both readers and the formatter.

## Output is always LF

Line endings are always normalized to `\n`: CRLF input is converted, and `.editorconfig`'s `end_of_line` is ignored.

Making this configurable is skipped for now, but will be added if there is demand.

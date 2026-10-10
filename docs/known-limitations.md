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

## `{% load %}` tags are left as written

Djangofmt never merges `{% load %}` tags nor sorts the libraries they load.
djangofmt keeps one library per `{% load %}` line, and reordering libraries can change which tag wins when two of them define the same name.

## Template tags inside text

Django compiles the `{% %}` inside an attribute value, an HTML comment or a `<script>`, `<style>`, `<pre>` or `<textarea>` body like any other, but the parser keeps those as text.
The formatter prints such tags and the `{{ }}` next to them as written; `style` values are the exception, their `{{ }}` are normalized along with the CSS.

```html
<a href="{%  url  'home' %}" title="{{ page.title | title }}">Home</a>
```

Lint rules read each of those tags on its own, so a block written there, or one whose opening and closing tags sit in two such places, is not paired, and the rules that need both tags skip it, such as [`deprecated-ifequal-tag`](rules/deprecated-ifequal-tag.md), [`missing-endblock-label`](rules/missing-endblock-label.md) and [`redundant-endblock-label`](rules/redundant-endblock-label.md).
[`legacy-translation-tag`](rules/legacy-translation-tag.md) is the exception: it finds a `{% blocktrans %}`'s closer the way Django does.

```html
<option title="{% ifequal a b %}Current{% endifequal %}">Home</option>
```

A block in attribute position, such as `<option {% ifequal a b %}selected{% endifequal %}>`, is paired as usual.

## Multi-line template tags

Up to Django 6.1, a `{% %}` tag must fit on one line: Django reads one that spans several lines as plain text.
Lint rules skip such a tag inside the text above, and the formatter joins one written in the template body onto a single line, turning text Django printed as is into a tag.

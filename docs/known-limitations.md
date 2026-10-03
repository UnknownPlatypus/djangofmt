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
One library per `{% load %}` line is the preferred style, and reordering libraries can change which tag wins when two of them define the same name.

[djade](https://github.com/adamchainz/djade) merges and sorts them, so running djade after djangofmt still rewrites `{% load %}` lines.
See [Coming from djade](djade.md) for the rest of djade's rules.

## Template tags inside attribute values

The formatter normalizes the whitespace inside `{% %}` and `{{ }}`, but not inside an attribute value, where both are printed as written.
`style` values are the exception: their `{{ }}` are normalized along with the CSS.

```html
<a href="{%  url  'home' %}" title="{{ page.title | title }}">Home</a>
```

Lint rules still read the `{% %}` tags of attribute values, HTML comments and `<script>`, `<style>`, `<pre>` and `<textarea>` bodies, since Django compiles them like any other.

## Blocks inside attribute values are not paired

Inside an attribute value, an HTML comment or a `<script>`, `<style>`, `<pre>` or `<textarea>` body, lint rules see each `{% %}` tag on its own.
A block written there, or one whose opening and closing tags sit in two such places, is not paired, so the rules that need both tags skip it:
[`deprecated-ifequal-tag`](rules/deprecated-ifequal-tag.md), [`missing-endblock-label`](rules/missing-endblock-label.md) and [`redundant-endblock-label`](rules/redundant-endblock-label.md).

```html
<option title="{% ifequal a b %}Current{% endifequal %}">Home</option>
```

A block in attribute position, such as `<option {% ifequal a b %}selected{% endifequal %}>`, is paired as usual.

## Multi-line template tags

Up to Django 6.1, a `{% %}` tag must fit on one line: Django reads one that spans several lines as plain text.
Multi-line tags may come in a later Django release ([django/django#21982](https://github.com/django/django/pull/21982)), and djangofmt does not support them yet:
lint rules skip such a tag inside an attribute value, an HTML comment or a `<script>`, `<style>`, `<pre>` or `<textarea>` body, and the formatter joins one written in the template body onto a single line.

## Whitespace control in comments

Under the `django` profile, the formatter puts one space at each end of a `{# #}` comment.
Django has no whitespace control, so the dashes of a Jinja-style `{#- … -#}` comment are part of its text and get spaced out:

```diff
-{#- Sidebar -#}
+{# - Sidebar - #}
```

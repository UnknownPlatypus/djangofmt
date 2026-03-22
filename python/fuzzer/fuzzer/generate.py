from __future__ import annotations

import random
import string
from typing import ClassVar


class TemplateGenerator:
    """Generates random Django/Jinja templates deterministically from a seed."""

    HTML_ELEMENTS: ClassVar[list[str]] = [
        "div",
        "p",
        "span",
        "a",
        "ul",
        "ol",
        "li",
        "table",
        "tr",
        "td",
        "th",
        "form",
        "section",
        "article",
        "header",
        "footer",
        "nav",
        "main",
        "h1",
        "h2",
        "h3",
        "strong",
        "em",
        "label",
        "textarea",
        "select",
        "option",
        "button",
    ]
    VOID_ELEMENTS: ClassVar[list[str]] = ["input", "br", "hr", "img", "meta", "link"]
    ATTRIBUTES: ClassVar[list[str]] = [
        "class",
        "id",
        "style",
        "href",
        "src",
        "alt",
        "title",
        "name",
        "type",
        "value",
        "placeholder",
        "data-id",
        "data-toggle",
        "aria-label",
        "role",
    ]
    VARIABLE_NAMES: ClassVar[list[str]] = [
        "user",
        "item",
        "obj",
        "request",
        "form",
        "page",
        "post",
        "comment",
        "name",
        "title",
        "count",
        "items",
        "data",
        "value",
        "content",
    ]
    VARIABLE_ATTRS: ClassVar[list[str]] = [
        "name",
        "id",
        "pk",
        "title",
        "email",
        "username",
        "get_absolute_url",
        "count",
        "is_active",
        "0",
        "1",
    ]
    FILTER_NAMES: ClassVar[list[str]] = [
        "lower",
        "upper",
        "title",
        "capfirst",
        "escape",
        "safe",
        "length",
        "default",
        "truncatewords",
        "date",
        "join",
        "linebreaks",
        "striptags",
        "urlencode",
        "slugify",
        "yesno",
        "first",
        "last",
        "add",
    ]
    FILTER_ARGS: ClassVar[list[str]] = [
        "'fallback'",
        "'10'",
        "'Y-m-d'",
        "', '",
        "'yes,no'",
        "'3'",
    ]
    TAG_NAMES: ClassVar[list[str]] = [
        "my_tags",
        "i18n",
        "l10n",
        "static",
        "cache",
        "humanize",
        "tz",
    ]
    URL_NAMES: ClassVar[list[str]] = [
        "home",
        "detail",
        "list",
        "create",
        "update",
        "delete",
        "login",
        "logout",
        "profile",
        "search",
    ]
    BLOCK_NAMES: ClassVar[list[str]] = [
        "content",
        "title",
        "header",
        "footer",
        "sidebar",
        "scripts",
        "styles",
        "extra_head",
        "breadcrumbs",
        "main",
    ]
    TEMPLATE_NAMES: ClassVar[list[str]] = [
        "'base.html'",
        "'header.html'",
        "'footer.html'",
        "'sidebar.html'",
        "'partials/nav.html'",
        "'includes/form.html'",
    ]

    def __init__(
        self, seed: int, *, profile: str = "django", max_depth: int = 6
    ) -> None:
        self.rng = random.Random(seed)
        self.profile = profile
        self.max_depth = max_depth

    def generate(self) -> str:
        """Generate a random Django template string."""
        lines: list[str] = []

        # Optionally start with an extends tag
        if self.rng.random() < 0.15:
            lines.append(self._extends_tag())
            lines.append("")

        # Optionally add load tags
        if self.rng.random() < 0.3:
            num_loads = self.rng.randint(1, 3)
            for _ in range(num_loads):
                lines.append(self._load_tag())
            lines.append("")

        # Generate body content
        num_nodes = self.rng.randint(1, 10)
        for _ in range(num_nodes):
            lines.extend(self._node(depth=0))

        return "\n".join(lines)

    def _node(self, depth: int) -> list[str]:
        """Generate a random template node (HTML, tag, variable, comment, etc.)."""
        if depth >= self.max_depth:
            return [self._leaf_node()]

        # Weighted random selection of node types
        block_generators = [self._html_element_node, self._django_block_node]
        leaf_generators = [
            self._variable_line,
            self._comment_line,
            self._plain_text_line,
            self._standalone_tag_line,
            self._void_element_line,
            self._mixed_content_line,
        ]
        all_weights = [30, 20, 15, 5, 10, 10, 5, 5]
        idx = self.rng.choices(range(len(all_weights)), weights=all_weights, k=1)[0]

        if idx < len(block_generators):
            return block_generators[idx](depth)
        return [leaf_generators[idx - len(block_generators)]()]

    def _leaf_node(self) -> str:
        choice = self.rng.randint(0, 3)
        if choice == 0:
            return self._variable_expr()
        if choice == 1:
            return self._plain_text()
        if choice == 2:
            return self._standalone_tag()
        return self._comment()

    # --- HTML elements ---

    def _html_element_node(self, depth: int) -> list[str]:
        tag = self.rng.choice(self.HTML_ELEMENTS)
        attrs = self._html_attrs()
        indent = "  " * depth

        children_lines: list[str] = []
        num_children = self.rng.randint(0, 4)
        for _ in range(num_children):
            children_lines.extend(self._node(depth + 1))

        if not children_lines:
            # Sometimes emit empty element
            return [f"{indent}<{tag}{attrs}></{tag}>"]

        lines = [f"{indent}<{tag}{attrs}>"]
        for child in children_lines:
            lines.append(f"{indent}  {child}")
        lines.append(f"{indent}</{tag}>")
        return lines

    def _void_element_line(self) -> str:
        tag = self.rng.choice(self.VOID_ELEMENTS)
        attrs = self._html_attrs()
        if self.rng.random() < 0.3:
            return f"<{tag}{attrs} />"
        return f"<{tag}{attrs}>"

    def _html_attrs(self) -> str:
        if self.rng.random() < 0.3:
            return ""
        num_attrs = self.rng.randint(1, 3)
        attrs = []
        chosen = self.rng.sample(self.ATTRIBUTES, min(num_attrs, len(self.ATTRIBUTES)))
        for attr in chosen:
            if self.rng.random() < 0.2:
                # Attribute with template variable value
                attrs.append(f'{attr}="{self._variable_expr()}"')
            else:
                val = self._random_word()
                attrs.append(f'{attr}="{val}"')
        return " " + " ".join(attrs)

    # --- Django template tags ---

    def _django_block_node(self, depth: int) -> list[str]:
        block_types = ["if", "for", "block", "with", "comment", "spaceless"]
        if self.profile == "jinja":
            block_types.extend(["macro", "raw"])

        block_type = self.rng.choice(block_types)
        indent = "  " * depth

        children_lines: list[str] = []
        num_children = self.rng.randint(0, 4)
        for _ in range(num_children):
            children_lines.extend(self._node(depth + 1))

        if block_type == "if":
            return self._if_block(indent, children_lines)
        if block_type == "for":
            return self._for_block(indent, children_lines)
        if block_type == "block":
            return self._block_block(indent, children_lines)
        if block_type == "with":
            return self._with_block(indent, children_lines)
        if block_type == "comment":
            return self._comment_block(indent, children_lines)
        if block_type == "spaceless":
            return self._spaceless_block(indent, children_lines)
        if block_type == "macro":
            return self._macro_block(indent, children_lines)
        if block_type == "raw":
            return self._raw_block(indent, children_lines)
        return children_lines  # pragma: no cover

    def _if_block(self, indent: str, children: list[str]) -> list[str]:
        cond = self._condition()
        lines = [f"{indent}{{% if {cond} %}}"]
        for c in children:
            lines.append(f"{indent}  {c}")

        # Optionally add elif/else
        if self.rng.random() < 0.3:
            lines.append(f"{indent}{{% elif {self._condition()} %}}")
            lines.append(f"{indent}  {self._leaf_node()}")
        if self.rng.random() < 0.4:
            lines.append(f"{indent}{{% else %}}")
            lines.append(f"{indent}  {self._leaf_node()}")

        lines.append(f"{indent}{{% endif %}}")
        return lines

    def _for_block(self, indent: str, children: list[str]) -> list[str]:
        loop_var = self.rng.choice(
            ["item", "obj", "x", "entry", "element", "row", "child"]
        )
        iterable = self.rng.choice(self.VARIABLE_NAMES)
        lines = [f"{indent}{{% for {loop_var} in {iterable} %}}"]
        for c in children:
            lines.append(f"{indent}  {c}")

        if self.rng.random() < 0.2:
            lines.append(f"{indent}{{% empty %}}")
            lines.append(f"{indent}  <p>No items found.</p>")

        lines.append(f"{indent}{{% endfor %}}")
        return lines

    def _block_block(self, indent: str, children: list[str]) -> list[str]:
        name = self.rng.choice(self.BLOCK_NAMES)
        lines = [f"{indent}{{% block {name} %}}"]
        for c in children:
            lines.append(f"{indent}  {c}")
        lines.append(f"{indent}{{% endblock %}}")
        return lines

    def _with_block(self, indent: str, children: list[str]) -> list[str]:
        var = self.rng.choice(self.VARIABLE_NAMES)
        alias = self._random_word()
        lines = [
            f"{indent}{{% with {alias}={var}.{self.rng.choice(self.VARIABLE_ATTRS)} %}}"
        ]
        for c in children:
            lines.append(f"{indent}  {c}")
        lines.append(f"{indent}{{% endwith %}}")
        return lines

    def _comment_block(self, indent: str, children: list[str]) -> list[str]:
        lines = [f"{indent}{{% comment %}}"]
        for c in children:
            lines.append(f"{indent}  {c}")
        lines.append(f"{indent}{{% endcomment %}}")
        return lines

    def _spaceless_block(self, indent: str, children: list[str]) -> list[str]:
        lines = [f"{indent}{{% spaceless %}}"]
        for c in children:
            lines.append(f"{indent}  {c}")
        lines.append(f"{indent}{{% endspaceless %}}")
        return lines

    def _macro_block(self, indent: str, children: list[str]) -> list[str]:
        name = self._random_word()
        args = ", ".join(self._random_word() for _ in range(self.rng.randint(0, 3)))
        lines = [f"{indent}{{% macro {name}({args}) %}}"]
        for c in children:
            lines.append(f"{indent}  {c}")
        lines.append(f"{indent}{{% endmacro %}}")
        return lines

    def _raw_block(self, indent: str, children: list[str]) -> list[str]:
        lines = [f"{indent}{{% raw %}}"]
        lines.append(f"{indent}  {{{{ this_should_not_be_parsed }}}}")
        for c in children:
            lines.append(f"{indent}  {c}")
        lines.append(f"{indent}{{% endraw %}}")
        return lines

    # --- Template variables ---

    def _variable_expr(self) -> str:
        var = self.rng.choice(self.VARIABLE_NAMES)
        # Optionally add attribute access
        if self.rng.random() < 0.5:
            var += "." + self.rng.choice(self.VARIABLE_ATTRS)
        # Optionally add filters
        num_filters = 0
        if self.rng.random() < 0.4:
            num_filters = self.rng.randint(1, 3)
        for _ in range(num_filters):
            f = self.rng.choice(self.FILTER_NAMES)
            if self.rng.random() < 0.3 and f in (
                "default",
                "truncatewords",
                "date",
                "join",
                "yesno",
                "add",
            ):
                arg = self.rng.choice(self.FILTER_ARGS)
                var += f"|{f}:{arg}"
            else:
                var += f"|{f}"

        # Whitespace control for jinja
        if self.profile == "jinja" and self.rng.random() < 0.2:
            return f"{{{{- {var} -}}}}"
        return f"{{{{ {var} }}}}"

    def _variable_line(self) -> str:
        return self._variable_expr()

    # --- Comments ---

    def _comment(self) -> str:
        if self.rng.random() < 0.7:
            return f"{{# {self._random_text()} #}}"
        return f"<!-- {self._random_text()} -->"

    def _comment_line(self) -> str:
        return self._comment()

    # --- Standalone tags ---

    def _standalone_tag(self) -> str:
        tag_types = ["csrf_token", "url", "include", "trans", "load", "static"]
        tag_type = self.rng.choice(tag_types)

        if tag_type == "csrf_token":
            return "{% csrf_token %}"
        if tag_type == "url":
            url_name = self.rng.choice(self.URL_NAMES)
            if self.rng.random() < 0.3:
                return f"{{% url '{url_name}' {self.rng.choice(self.VARIABLE_NAMES)}.pk %}}"
            return f"{{% url '{url_name}' %}}"
        if tag_type == "include":
            return f"{{% include {self.rng.choice(self.TEMPLATE_NAMES)} %}}"
        if tag_type == "trans":
            return f'{{% trans "{self._random_text()}" %}}'
        if tag_type == "load":
            return self._load_tag()
        # static
        return f"{{% static 'css/{self._random_word()}.css' %}}"

    def _standalone_tag_line(self) -> str:
        return self._standalone_tag()

    def _extends_tag(self) -> str:
        return f"{{% extends {self.rng.choice(self.TEMPLATE_NAMES)} %}}"

    def _load_tag(self) -> str:
        return f"{{% load {self.rng.choice(self.TAG_NAMES)} %}}"

    # --- Plain text / mixed content ---

    def _plain_text(self) -> str:
        return self._random_text()

    def _plain_text_line(self) -> str:
        return self._plain_text()

    def _mixed_content_line(self) -> str:
        """Generate a line mixing text, variables, and small HTML."""
        parts: list[str] = []
        num_parts = self.rng.randint(2, 5)
        for _ in range(num_parts):
            choice = self.rng.randint(0, 3)
            if choice == 0:
                parts.append(self._random_text())
            elif choice == 1:
                parts.append(self._variable_expr())
            elif choice == 2:
                tag = self.rng.choice(["strong", "em", "span", "a"])
                parts.append(f"<{tag}>{self._variable_expr()}</{tag}>")
            else:
                parts.append(self._standalone_tag())
        return " ".join(parts)

    # --- Conditions ---

    def _condition(self) -> str:
        var = self.rng.choice(self.VARIABLE_NAMES)
        if self.rng.random() < 0.3:
            var += "." + self.rng.choice(self.VARIABLE_ATTRS)

        cond_type = self.rng.randint(0, 4)
        if cond_type == 0:
            return var
        if cond_type == 1:
            return f"not {var}"
        if cond_type == 2:
            other = self.rng.choice(self.VARIABLE_NAMES)
            return f"{var} == {other}"
        if cond_type == 3:
            return f'{var} == "{self._random_word()}"'
        return f"{var} in {self.rng.choice(self.VARIABLE_NAMES)}"

    # --- Helpers ---

    def _random_word(self) -> str:
        length = self.rng.randint(3, 10)
        return "".join(self.rng.choices(string.ascii_lowercase, k=length))

    def _random_text(self) -> str:
        num_words = self.rng.randint(1, 6)
        return " ".join(self._random_word() for _ in range(num_words))

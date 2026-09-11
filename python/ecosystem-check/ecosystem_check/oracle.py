"""
Parse a template with Django or Jinja, accepting unknown tags, filters and libraries.
"""

from __future__ import annotations

from collections import defaultdict
from collections.abc import Callable
from typing import Any

import jinja2
from django.template import Engine, Origin, TemplateSyntaxError
from django.template.base import Lexer, Node, Parser, TextNode, Token
from django.template.library import Library

JINJA_ENV = jinja2.Environment(
    extensions=[
        "jinja2.ext.i18n",
        "jinja2.ext.do",
        "jinja2.ext.loopcontrols",
        "jinja2.ext.debug",
    ]
)
# Parsing never reads settings, so neither `settings.configure()` nor `django.setup()` is needed.
DJANGO_ENGINE = Engine()


def noop_tag(parser: Parser, token: Token) -> Node:
    return TextNode("")


def noop_filter(value: Any, arg: Any = None) -> Any:
    return value


class PermissiveParser(Parser):
    """Compiles unknown tags, filters and libraries to no-ops, so only Django's own grammar is checked."""

    def __init__(
        self,
        tokens: list[Token],
        libraries: dict[str, Library],
        builtins: list[Library],
        origin: Origin,
    ) -> None:
        super().__init__(tokens, libraries, builtins, origin)
        self.tags = defaultdict(lambda: noop_tag, self.tags)
        self.libraries = defaultdict(Library, self.libraries)

    def find_filter(self, filter_name: str) -> Callable[..., Any]:
        return self.filters.get(filter_name, noop_filter)


def parse_django(source: str, template_name: str) -> str | None:
    """Return the parse error, or `None` when the template parses."""
    # Relative `{% include %}` and `{% extends %}` paths resolve against the origin's template name.
    origin = Origin(name=template_name, template_name=template_name)
    parser = PermissiveParser(
        Lexer(source).tokenize(),
        DJANGO_ENGINE.template_libraries,
        DJANGO_ENGINE.template_builtins,
        origin,
    )
    try:
        parser.parse()
    except TemplateSyntaxError as e:
        return str(e)
    return None


def parse_jinja(source: str, template_name: str) -> str | None:
    try:
        JINJA_ENV.parse(source, name=template_name)
    except jinja2.TemplateSyntaxError as e:
        return str(e)
    return None

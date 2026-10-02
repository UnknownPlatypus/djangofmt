use std::iter;
use std::slice;
use std::str::SplitAsciiWhitespace;
use std::sync::LazyLock;

use djangofmt_syntax::CommentDelimiters;
use djangofmt_syntax::dtl::{first_bit_is, is_space};
use markup_fmt::ast::{Attribute, JinjaBlock, JinjaTag, JinjaTagOrChildren, NativeAttribute};
use memchr::memmem::Finder;
use memchr::{memchr, memchr_iter, memrchr};
use smallvec::{SmallVec, smallvec};

use crate::Checker;

/// Returns true if the value contains Jinja/Django template markers.
///
/// Values with `{{`, `{%` or `{#` are dynamic and should be skipped by most rules.
#[inline]
pub fn contains_interpolation(value: &str) -> bool {
    value.contains("{{") || value.contains("{%") || value.contains("{#")
}

/// The whitespace-separated tokens of a tag's content: `{% block NAME %}` yields `block`, `NAME`.
/// Strip `{%-`/`{%+` markers first; otherwise they become a leading token and shift the rest.
pub fn tag_tokens(content: &str) -> SplitAsciiWhitespace<'_> {
    content
        .trim_start_matches(['+', '-'])
        .split_ascii_whitespace()
}

/// A block whose closing tag Django lets repeat the block's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// `{% block NAME %}`
    Block,
    /// `{% partialdef NAME %}`
    Partialdef,
}

impl BlockKind {
    /// The closing tag's name.
    #[must_use]
    pub const fn end_tag(self) -> &'static str {
        match self {
            Self::Block => "endblock",
            Self::Partialdef => "endpartialdef",
        }
    }
}

/// The closing tag of a `{% block NAME %}` or `{% partialdef NAME %}`, in a form Django accepts.
pub struct BlockEnd<'s> {
    pub kind: BlockKind,
    /// The block's name, from the opening tag.
    pub name: &'s str,
    /// `endblock` or `endpartialdef`, in the closing tag.
    pub tag_name: &'s str,
    /// The closing tag's label, which repeats `name`.
    pub label: Option<&'s str>,
    /// Whether the closing tag sits on the opening tag's line.
    pub same_line: bool,
}

/// The [`BlockEnd`] of `block`, or [`None`] for any other block and for the tags Django rejects:
/// another arity, or a label naming another block.
pub fn block_end<'s, T>(checker: &Checker<'s>, block: &JinjaBlock<'s, T>) -> Option<BlockEnd<'s>> {
    let (Some(JinjaTagOrChildren::Tag(opener)), Some(JinjaTagOrChildren::Tag(closer))) =
        (block.body.first(), block.body.last())
    else {
        return None;
    };

    let mut tokens = tag_tokens(opener.content);
    let kind = match tokens.next()? {
        "block" => BlockKind::Block,
        "partialdef" => BlockKind::Partialdef,
        _ => return None,
    };
    let name = match (tokens.next()?, tokens.next(), tokens.next()) {
        (name, None, _) => name,
        (name, Some("inline"), None) if kind == BlockKind::Partialdef => name,
        _ => return None,
    };

    let mut tokens = tag_tokens(closer.content);
    let tag_name = tokens.next().filter(|&token| token == kind.end_tag())?;
    let label = match (tokens.next(), tokens.next()) {
        (None, _) => None,
        (Some(label), None) if label == name => Some(label),
        _ => return None,
    };

    let between = &checker.context().source()
        [checker.source_end(opener.content)..checker.source_offset(closer.content)];
    Some(BlockEnd {
        kind,
        name,
        tag_name,
        label,
        same_line: !between.contains('\n'),
    })
}

/// A [django-cotton](https://django-cotton.com) component (`<c-button>`), whose attributes and
/// content are the component's API rather than HTML. Cotton only compiles Django templates.
pub fn is_cotton_component(checker: &Checker<'_>, tag_name: &str) -> bool {
    checker.is_django() && tag_name.starts_with("c-")
}

/// Yields each `srcset` candidate URL.
///
/// `srcset` holds a comma-separated list of candidates, each `<url> <descriptor>`
/// (e.g. `a.png 1x, b.png 2x`); the URL is the first whitespace-delimited token of
/// each candidate.
pub fn srcset_candidates(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(',')
        .filter_map(|candidate| candidate.split_ascii_whitespace().next())
}

/// Returns true if `attrs` declares a native HTML attribute named `name` (case-insensitive).
pub fn declares_native_attr(attrs: &[Attribute<'_>], name: &str) -> bool {
    native_attrs(attrs).any(|native| native.name.eq_ignore_ascii_case(name))
}

/// Returns true if `attrs` may render attributes the source doesn't show: a templated name such as
/// `{{ attrs }}`, or a tag such as `{% include "attrs.html" %}`, in any `{% if %}` branch too.
pub fn has_attr_spread(attrs: &[Attribute<'_>]) -> bool {
    attrs.iter().any(|attr| match attr {
        Attribute::Native(native) => contains_interpolation(native.name),
        Attribute::JinjaTag(_) => true,
        Attribute::JinjaBlock(block) => block.body.iter().any(|item| match item {
            JinjaTagOrChildren::Children(children) => has_attr_spread(children),
            JinjaTagOrChildren::Tag(_) => false,
        }),
        _ => false,
    })
}

/// Yields every native HTML attribute in `attrs`, in source order, descending into each branch
/// of a Jinja `{% if %}…{% endif %}` block.
///
/// Jinja `Tag` items yield nothing; we don't peek inside other tag bodies.
pub fn native_attrs<'a, 's>(
    attrs: &'a [Attribute<'s>],
) -> impl Iterator<Item = &'a NativeAttribute<'s>> {
    // Attribute lists left to walk, innermost last.
    let mut stack: SmallVec<[slice::Iter<'a, Attribute<'s>>; 4]> = smallvec![attrs.iter()];
    iter::from_fn(move || {
        loop {
            let Some(attr) = stack.last_mut()?.next() else {
                stack.pop();
                continue;
            };
            match attr {
                Attribute::Native(native) => return Some(native),
                // Reversed, so that the branches come out in source order.
                Attribute::JinjaBlock(block) => {
                    stack.extend(block.body.iter().rev().filter_map(|item| match item {
                        JinjaTagOrChildren::Children(children) => Some(children.iter()),
                        JinjaTagOrChildren::Tag(_) => None,
                    }));
                }
                _ => {}
            }
        }
    })
}

/// The whole comment around `comment_body`, delimiters included.
/// An unterminated comment runs to the end of the source.
pub fn enclosing_comment<'s>(
    checker: &Checker<'s>,
    delimiters: CommentDelimiters,
    comment_body: &str,
) -> &'s str {
    let source = checker.context().source();
    let start = checker.source_offset(comment_body) - delimiters.open.len();
    let end = checker.source_end(comment_body) + delimiters.close.len();
    &source[start..end.min(source.len())]
}

/// Built once: most texts are short, and a per-call searcher costs more than the scan.
static TAG_OPEN: LazyLock<Finder<'static>> = LazyLock::new(|| Finder::new(b"{%"));

/// The offset of the first `{%` in `text`.
pub fn find_tag_open(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    // Unlike `TAG_OPEN`, no searcher to set up, which the checker's many short searches feel.
    // `%` is rare outside tags, where `{` opens every `{{ }}` too.
    memchr_iter(b'%', bytes)
        .find(|&i| i > 0 && bytes[i - 1] == b'{')
        .map(|i| i - 1)
}

/// The `{% %}` tags Django reads in `text`, a run of the template the parser keeps verbatim, such
/// as an HTML comment, a `<script>` body or an attribute value. Each `start` is relative to `text`.
///
/// Mirrors Django's `tag_re`: a tag runs from `{%` to the first `%}` on the same line, and a `{%`
/// inside a `{{ }}` or `{# #}` is not one. The tags inside a `{% verbatim %}` or `{% comment %}`
/// body are dropped, their opener and closer kept; an unclosed one runs to the end of `text`.
pub fn tags_in_text(text: &str) -> impl Iterator<Item = JinjaTag<'_>> {
    let mut bodies = Bodies::default();
    lex_tags(text).filter(move |tag| bodies.compiles(tag.content))
}

/// Where a run of tags stands in Django's handling of `{% verbatim %}` and `{% comment %}` bodies.
#[derive(Default)]
struct Bodies<'a> {
    /// The lexer's state: the content of the open `{% verbatim %}`, whose body is text.
    verbatim: Option<&'a str>,
    /// The parser's state: inside a `{% comment %}` body, which `skip_past` drops.
    in_comment: bool,
}

impl<'a> Bodies<'a> {
    /// Whether Django compiles the tag holding `content`, given the tags before it.
    fn compiles(&mut self, content: &'a str) -> bool {
        // Outside a body only `verbatim` or `comment` opens one, so most tags pass on their first
        // byte. Rarer whitespace than ASCII may precede the name, and takes the full check.
        (self.verbatim.is_none()
            && !self.in_comment
            && !matches!(
                content.trim_ascii_start().as_bytes().first(),
                Some(b'v' | b'c' | b'\x0B' | b'\x1C'..=b'\x1F' | 0x80..)
            ))
            || self.compiles_slow(content)
    }

    #[cold]
    fn compiles_slow(&mut self, content: &'a str) -> bool {
        let content = content.trim_matches(is_space);
        // The lexer runs before the parser: a verbatim body hides an `{% endcomment %}` within it.
        if let Some(opener) = self.verbatim {
            // Only `end` plus the opener's content, `{% endverbatim x %}` for `{% verbatim x %}`.
            if content.strip_prefix("end") != Some(opener) {
                return false;
            }
            self.verbatim = None;
        } else if content == "verbatim" || content.starts_with("verbatim ") {
            self.verbatim = Some(content);
        }
        if self.in_comment {
            self.in_comment = content != "endcomment";
            return !self.in_comment;
        }
        self.in_comment = first_bit_is(content, "comment");
        true
    }
}

/// Every `{% %}` token of Django's lexer in `text`, before the verbatim and comment handling.
fn lex_tags(text: &str) -> impl Iterator<Item = JinjaTag<'_>> {
    let bytes = text.as_bytes();
    // Django's tokens never span a newline, so only the lines holding a `{%` are tokenized.
    let mut pos = 0;
    let mut line_end = 0;
    // Per closer, whether it is missing from the rest of the line: never searching twice for it
    // keeps the scan linear.
    let mut unclosed = [false; 3];
    iter::from_fn(move || {
        loop {
            if pos == line_end {
                let next_tag = pos + TAG_OPEN.find(&bytes[pos..])?;
                pos =
                    memrchr(b'\n', &bytes[pos..next_tag]).map_or(pos, |newline| pos + newline + 1);
                line_end = memchr(b'\n', &bytes[next_tag..])
                    .map_or(bytes.len(), |newline| next_tag + newline);
                unclosed = [false; 3];
            }
            let Some(brace) = memchr(b'{', &bytes[pos..line_end]) else {
                pos = line_end;
                continue;
            };
            let open = pos + brace;
            // Like the regex, a failed match retries from the next byte: `{{%` holds a tag when
            // `}}` never comes.
            pos = open + 1;
            let (close, kind) = match bytes.get(open + 1) {
                Some(b'%') => (b'%', 0),
                Some(b'{') => (b'}', 1),
                Some(b'#') => (b'#', 2),
                _ => continue,
            };
            if unclosed[kind] {
                continue;
            }
            let body = open + 2;
            let Some(len) = find_closer(&bytes[body..line_end], close) else {
                unclosed[kind] = true;
                continue;
            };
            pos = body + len + 2;
            if close == b'%' {
                return Some(JinjaTag {
                    content: &text[body..body + len],
                    start: body,
                });
            }
        }
    })
}

/// The offset in `line` of the first `close` byte followed by `}`.
fn find_closer(line: &[u8], close: u8) -> Option<usize> {
    memchr_iter(b'}', line.get(1..)?).find(|&i| line[i] == close)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::tags_in_text;

    #[rstest]
    #[case::tag("a {% load static %} b", &[(" load static ", 4)])]
    #[case::two_on_one_line("{% if a %}x{%endif%}", &[(" if a ", 2), ("endif", 13)])]
    #[case::first_closer_wins(r#"{% x "%}" %}"#, &[(r#" x ""#, 2)])]
    #[case::variable_and_comment_skipped("{{ '{% a %}' }} {# {% b %} #}", &[])]
    #[case::multi_line_not_a_tag("{% if\na %}", &[])]
    #[case::unterminated_not_a_tag("{% a", &[])]
    #[case::unclosed_variable_retries("{{% a %}", &[(" a ", 3)])]
    #[case::variable_ends_at_newline("{{ a\n{% b %} }}", &[(" b ", 7)])]
    #[case::verbatim_body_dropped(
        "{% verbatim x %}{% a %}{% endverbatim %}{% endverbatim x %}{% b %}",
        &[(" verbatim x ", 2), (" endverbatim x ", 42), (" b ", 61)]
    )]
    #[case::comment_body_dropped(
        r#"{% comment "note" %}{% a %}{% endcomment %}{% b %}"#,
        &[(r#" comment "note" "#, 2), (" endcomment ", 29), (" b ", 45)]
    )]
    fn tags_in_text_cases(#[case] text: &str, #[case] expected: &[(&str, usize)]) {
        let tags: Vec<_> = tags_in_text(text)
            .map(|tag| (tag.content, tag.start))
            .collect();
        assert_eq!(tags, expected);
    }
}

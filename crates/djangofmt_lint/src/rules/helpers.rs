use std::iter;
use std::slice;

use djangofmt_syntax::CommentDelimiters;
use djangofmt_syntax::dtl::{is_space, tag_name};
use markup_fmt::ast::{Attribute, JinjaTag, JinjaTagOrChildren, NativeAttribute};
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

/// The offset of the first `{%` in `bytes`: `%` is rare outside tags, where `{` opens every
/// `{{ }}` too, so it is the byte to search for.
fn find_tag_open(bytes: &[u8]) -> Option<usize> {
    memchr_iter(b'%', bytes)
        .find(|&i| i > 0 && bytes[i - 1] == b'{')
        .map(|i| i - 1)
}

/// The tags Django's `tag_re` reads in `text`, with each `start` relative to `text`; the ones in a
/// `{% verbatim %}` or `{% comment %}` body are dropped, their opener and closer kept.
pub fn tags_in_text(text: &str) -> impl Iterator<Item = JinjaTag<'_>> {
    let mut bodies = SkippedBody::default();
    lex_tags(text).filter(move |tag| bodies.compiles(tag.content))
}

/// Where a run of tags stands in Django's handling of `{% verbatim %}` and `{% comment %}` bodies.
#[derive(Default)]
struct SkippedBody<'a> {
    /// The lexer's state: the content of the open `{% verbatim %}`, whose body is text.
    verbatim: Option<&'a str>,
    /// The parser's state: inside a `{% comment %}` body, which `skip_past` drops.
    in_comment: bool,
}

impl<'a> SkippedBody<'a> {
    /// Whether Django compiles the tag holding `content`, given the tags before it.
    fn compiles(&mut self, content: &'a str) -> bool {
        // Outside a body only `verbatim` or `comment` opens one, so most tags pass on their first
        // byte.
        (self.verbatim.is_none()
            && !self.in_comment
            && !content.trim_start_matches(is_space).starts_with(['v', 'c']))
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
        self.in_comment = tag_name(content) == "comment";
        true
    }
}

/// The three closers `lex_tags` tracks per line.
const TAG: usize = 0;
const VARIABLE: usize = 1;
const COMMENT: usize = 2;

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
                let next_tag = pos + find_tag_open(&bytes[pos..])?;
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
                Some(b'%') => (b'%', TAG),
                Some(b'{') => (b'}', VARIABLE),
                Some(b'#') => (b'#', COMMENT),
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
    #[case::two_on_one_line("{% if a %}x{%endif%}", &[(" if a ", 2), ("endif", 13)])]
    #[case::first_closer_wins(r#"{% x "%}" %}"#, &[(r#" x ""#, 2)])]
    #[case::variable_and_comment_skipped("{{ '{% a %}' }} {# {% b %} #}", &[])]
    #[case::multi_line_not_a_tag("{% if\na %}", &[])]
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

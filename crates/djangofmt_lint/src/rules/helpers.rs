use std::iter;
use std::slice;
use std::str::SplitAsciiWhitespace;

use djangofmt_syntax::CommentDelimiters;
use markup_fmt::ast::{Attribute, JinjaBlock, JinjaTagOrChildren, NativeAttribute};
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

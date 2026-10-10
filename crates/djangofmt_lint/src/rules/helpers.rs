use std::iter;
use std::slice;

use djangofmt_syntax::CommentDelimiters;
use djangofmt_syntax::dtl::{bits, is_space};
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

/// Django's `token.split_contents()` for a tag whose arguments hold no quotes: `str.split()`.
pub fn tag_tokens(content: &str) -> impl Iterator<Item = &str> {
    content.split(is_space).filter(|token| !token.is_empty())
}

/// A block whose closing tag Django lets repeat the block's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Block,
    Partialdef,
}

impl BlockKind {
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
    pub name: &'s str,
    /// `endblock` or `endpartialdef`, in the closing tag.
    pub end_tag: &'s str,
    /// The closing tag's label, which repeats `name`.
    pub label: Option<&'s str>,
    pub same_line: bool,
}

/// The [`BlockEnd`] of a block the caller has classified as `kind`, or [`None`] for the tags
/// Django rejects: another arity, or a closing tag that is neither bare nor followed by the name.
pub fn block_end<'s, T>(
    checker: &Checker<'s>,
    block: &JinjaBlock<'s, T>,
    kind: BlockKind,
) -> Option<BlockEnd<'s>> {
    let (Some(JinjaTagOrChildren::Tag(opener)), Some(JinjaTagOrChildren::Tag(closer))) =
        (block.body.first(), block.body.last())
    else {
        return None;
    };

    let name = match kind {
        BlockKind::Block => {
            let mut tokens = tag_tokens(opener.content).skip(1);
            match (tokens.next()?, tokens.next()) {
                (name, None) => name,
                _ => return None,
            }
        }
        // `partialdef` splits with `split_contents()`, which keeps a quoted name whole.
        BlockKind::Partialdef => match bits(opener.content)[..] {
            [_, name] | [_, name, "inline"] => name,
            _ => return None,
        },
    };

    // Django compares the whole closing tag, so exactly one space separates the label.
    let end = closer.content.trim_matches(is_space);
    let rest = end.strip_prefix(kind.end_tag())?;
    let end_tag = &end[..kind.end_tag().len()];
    let label = match rest.strip_prefix(' ') {
        None if rest.is_empty() => None,
        Some(label) if label == name => Some(label),
        _ => return None,
    };

    let between = &checker.context().source()
        [checker.source_end(opener.content)..checker.source_offset(closer.content)];
    Some(BlockEnd {
        kind,
        name,
        end_tag,
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

use std::iter;
use std::slice;

use markup_fmt::ast::{Attribute, JinjaTagOrChildren, NativeAttribute};
use smallvec::{SmallVec, smallvec};

use crate::Checker;

/// Returns true if the value contains Jinja/Django interpolation markers.
///
/// Values with `{{` or `{%` are dynamic and should be skipped by most rules.
#[inline]
pub fn contains_interpolation(value: &str) -> bool {
    value.contains("{{") || value.contains("{%")
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

/// A UTF-8 BOM is not Rust whitespace, so strip it explicitly.
#[must_use]
#[inline]
pub fn strip_bom(source: &str) -> &str {
    source.strip_prefix('\u{feff}').unwrap_or(source)
}

/// The delimiters of one comment style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommentDelimiters {
    pub open: &'static str,
    pub close: &'static str,
}

/// A `{# #}` template comment, the only kind that carries a lint suppression.
pub const TEMPLATE_COMMENT: CommentDelimiters = CommentDelimiters {
    open: "{#",
    close: "#}",
};

/// An `<!-- -->` HTML comment, which is rendered to the client.
pub const HTML_COMMENT: CommentDelimiters = CommentDelimiters {
    open: "<!--",
    close: "-->",
};

impl CommentDelimiters {
    /// The body of a comment, only if `text` starts with one.
    #[inline]
    pub fn body(self, text: &str) -> Option<&str> {
        let body = text.strip_prefix(self.open)?;
        Some(&body[..body.find(self.close)?])
    }

    /// The whole comment around `comment_body`, delimiters included.
    /// An unterminated comment runs to the end of the source.
    pub fn enclosing_comment<'s>(self, checker: &Checker<'s>, comment_body: &str) -> &'s str {
        let source = checker.context().source();
        let start = checker.source_offset(comment_body) - self.open.len();
        let end = checker.source_end(comment_body) + self.close.len();
        &source[start..end.min(source.len())]
    }
}

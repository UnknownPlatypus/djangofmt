use std::ops::Range;

use markup_fmt::ast::{Attribute, JinjaBlock, JinjaTagOrChildren};

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

/// Returns true if `attr` declares a native HTML attribute named `name`
/// (case-insensitive), either directly or recursively inside any branch of a
/// Jinja `{% if %}…{% endif %}` block.
///
/// Jinja `Tag` items are treated as non-declaring; we don't peek inside other
/// tag bodies.
pub fn declares_native_attr(attr: &Attribute<'_>, name: &str) -> bool {
    match attr {
        Attribute::Native(native) => native.name.eq_ignore_ascii_case(name),
        Attribute::JinjaBlock(block) => jinja_block_declares_native_attr(block, name),
        _ => false,
    }
}

fn jinja_block_declares_native_attr(block: &JinjaBlock<'_, Attribute<'_>>, name: &str) -> bool {
    block.body.iter().any(|item| match item {
        JinjaTagOrChildren::Children(children) => {
            children.iter().any(|attr| declares_native_attr(attr, name))
        }
        JinjaTagOrChildren::Tag(_) => false,
    })
}

/// A UTF-8 BOM is not Rust whitespace, so strip it explicitly.
#[must_use]
#[inline]
pub fn strip_bom(source: &str) -> &str {
    source.strip_prefix('\u{feff}').unwrap_or(source)
}

/// The delimiters of one comment style.
#[derive(Debug, Clone, Copy)]
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
    /// The body of a leading comment, if `text` starts with one.
    #[inline]
    pub fn leading_body(self, text: &str) -> Option<&str> {
        let body = text.strip_prefix(self.open)?;
        Some(&body[..body.find(self.close)?])
    }

    /// The byte range of the whole comment around `body`, delimiters included.
    /// An unterminated comment runs to the end of the source.
    pub fn enclosing_range(self, checker: &Checker<'_>, body: &str) -> Range<usize> {
        let start = checker.source_offset(body) - self.open.len();
        let end = checker.source_end(body) + self.close.len();
        start..end.min(checker.context().source().len())
    }
}

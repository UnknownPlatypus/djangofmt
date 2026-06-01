use markup_fmt::ast::{Attribute, JinjaBlock, JinjaTagOrChildren};

/// Returns true if the value contains Jinja/Django interpolation markers.
///
/// Values with `{{` or `{%` are dynamic and should be skipped by most rules.
#[inline]
pub fn contains_interpolation(value: &str) -> bool {
    value.contains("{{") || value.contains("{%")
}

/// Yields each `srcset` candidate URL with its byte offset in the source.
///
/// `srcset` holds a comma-separated list of candidates, each `<url> <descriptor>`
/// (e.g. `a.png 1x, b.png 2x`); the URL is the first whitespace-delimited token of
/// each candidate. `base` is the byte offset of `value` in the source, so the
/// yielded offset points at the candidate URL itself.
pub fn srcset_candidates(value: &str, base: usize) -> impl Iterator<Item = (&str, usize)> {
    let mut pos = 0;
    value.split(',').filter_map(move |candidate| {
        let start = pos;
        pos += candidate.len() + 1; // skip the candidate and its `,`
        let trimmed = candidate.trim_start_matches(|c: char| c.is_ascii_whitespace());
        let url = trimmed.split_ascii_whitespace().next()?;
        Some((url, base + start + candidate.len() - trimmed.len()))
    })
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

/// Walk backwards from `offset` over ASCII whitespace bytes in `source`,
/// returning the offset of the first non-whitespace byte.
///
/// Used when deleting an attribute to absorb the leading whitespace separating
/// it from the previous token, so removing the only attribute leaves `<div>`
/// rather than `<div >`.
#[inline]
pub fn reverse_consume_ws(source: &[u8], mut offset: usize) -> usize {
    while offset > 0 && source[offset - 1].is_ascii_whitespace() {
        offset -= 1;
    }
    offset
}

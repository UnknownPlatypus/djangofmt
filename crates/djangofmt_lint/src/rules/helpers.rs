use markup_fmt::ast::{Attribute, JinjaBlock, JinjaTagOrChildren};

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

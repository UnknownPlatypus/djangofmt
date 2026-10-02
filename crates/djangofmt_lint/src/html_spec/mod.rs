//! Attribute value constraints from the HTML specification.

use std::cmp::Ordering;

mod generated;

use generated::{ATTRS, HTML_GLOBAL_ATTR_TAGS};

/// One attribute's keywords, per element and on every element taking the HTML global attributes.
struct Attr {
    name: &'static str,
    /// Sorted by tag.
    elements: &'static [(&'static str, &'static [&'static str])],
    global: Option<&'static [&'static str]>,
}

/// The keywords `attr` accepts on `tag`, when the spec restricts it to a fixed set.
#[must_use]
pub fn keywords(tag: &str, attr: &str) -> Option<&'static [&'static str]> {
    let index = ATTRS
        .binary_search_by(|entry| cmp_name(entry.name, attr))
        .ok()?;
    let entry = &ATTRS[index];
    if let Ok(index) = entry
        .elements
        .binary_search_by(|&(key, _)| cmp_name(key, tag))
    {
        return Some(entry.elements[index].1);
    }
    entry.global.filter(|_| {
        HTML_GLOBAL_ATTR_TAGS
            .binary_search_by(|key| cmp_name(key, tag))
            .is_ok()
    })
}

/// Whether `value` is one of `keywords`, which HTML matches ASCII case-insensitively.
#[must_use]
pub fn is_keyword(keywords: &[&str], value: &str) -> bool {
    keywords
        .iter()
        .any(|keyword| keyword.eq_ignore_ascii_case(value))
}

/// Order a lowercase table `key` against a tag or attribute `name`, whose case HTML ignores.
fn cmp_name(key: &str, name: &str) -> Ordering {
    key.bytes()
        .cmp(name.bytes().map(|byte| byte.to_ascii_lowercase()))
}

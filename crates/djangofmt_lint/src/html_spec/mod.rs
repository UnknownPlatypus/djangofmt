//! Attribute value constraints from the HTML specification.

use std::borrow::Cow;

mod generated;

use generated::{ATTRS, HTML_GLOBAL_ATTR_TAGS};

/// An attribute whose value must be one of a fixed set of keywords.
pub struct EnumAttr {
    pub values: &'static [&'static str],
    pub case_insensitive: bool,
}

impl EnumAttr {
    #[must_use]
    pub fn accepts(&self, value: &str) -> bool {
        self.values.iter().any(|keyword| {
            if self.case_insensitive {
                keyword.eq_ignore_ascii_case(value)
            } else {
                *keyword == value
            }
        })
    }
}

/// One attribute's keywords, per element and on every element taking the HTML global attributes.
struct Attr {
    name: &'static str,
    /// Sorted by tag. `None` when the element gives the attribute a type other than an enum.
    elements: &'static [(&'static str, Option<&'static EnumAttr>)],
    global: Option<&'static EnumAttr>,
}

/// The keywords `attr` accepts on `tag`, when the spec restricts it to a fixed set.
#[must_use]
pub fn enum_attr(tag: &str, attr: &str) -> Option<&'static EnumAttr> {
    let attr = ascii_lowercase(attr);
    let index = ATTRS
        .binary_search_by_key(&&*attr, |entry| entry.name)
        .ok()?;
    let entry = &ATTRS[index];
    let tag = ascii_lowercase(tag);
    if let Ok(index) = entry.elements.binary_search_by_key(&&*tag, |&(tag, _)| tag) {
        return entry.elements[index].1;
    }
    entry
        .global
        .filter(|_| HTML_GLOBAL_ATTR_TAGS.binary_search(&&*tag).is_ok())
}

/// Tag and attribute names are ASCII case-insensitive, so only uppercase names pay for a copy.
fn ascii_lowercase(name: &str) -> Cow<'_, str> {
    if name.bytes().any(|byte| byte.is_ascii_uppercase()) {
        Cow::Owned(name.to_ascii_lowercase())
    } else {
        Cow::Borrowed(name)
    }
}

/// Returns true if the value contains Jinja/Django interpolation markers.
///
/// Values with `{{` or `{%` are dynamic and should be skipped by most rules.
#[inline]
pub fn contains_interpolation(value: &str) -> bool {
    value.contains("{{") || value.contains("{%")
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

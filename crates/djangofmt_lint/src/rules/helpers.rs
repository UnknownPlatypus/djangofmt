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

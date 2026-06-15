use crate::fix::{Edit, Fix};
use crate::lint_context::LintContext;

/// Builds a safe fix that deletes a whole native attribute.
///
/// Removes the attribute's name, `=`, and quoted value, absorbing the leading
/// whitespace that separates it from the previous token so deleting the only
/// attribute leaves `<div>` rather than `<div >`.
///
/// `name` is the attribute-name slice; `value_str` and `value_offset` locate the
/// value in the source; `quoted` indicates whether the value is wrapped in quotes
/// (so the closing quote is included in the deletion).
pub fn delete_attr_fix(
    ctx: &LintContext<'_>,
    name: &str,
    value_str: &str,
    value_offset: usize,
    quoted: bool,
) -> Fix {
    let name_start = ctx.source_offset(name);
    let attr_end = value_offset + value_str.len() + usize::from(quoted);
    let fix_start = reverse_consume_ws(ctx.source().as_bytes(), name_start);
    Fix::safe_edit(Edit::deletion((fix_start, attr_end - fix_start).into()))
}

/// Walk backwards from `offset` over ASCII whitespace bytes in `source`,
/// returning the offset of the first non-whitespace byte.
#[inline]
fn reverse_consume_ws(source: &[u8], offset: usize) -> usize {
    source[..offset]
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map_or(0, |i| i + 1)
}

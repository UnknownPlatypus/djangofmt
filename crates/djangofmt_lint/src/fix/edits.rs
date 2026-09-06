use crate::fix::{Edit, Fix};
use crate::lint_context::LintContext;
use crate::span;

/// Builds a safe fix that deletes a whole native attribute (e.g. `type="text/javascript"`).
///
/// Removes the full attribute name, `=` and value, absorbing the leading whitespace that
/// separates it from the previous token to avoid leaving `<div >` for solo attribute and have `<div>` instead.
///
/// `name` and `value_str` are the attribute's name and value slices.
/// `quoted` indicates whether the value is wrapped in quotes (to include it in the deletion).
pub fn delete_attr_fix(ctx: &LintContext<'_>, name: &str, value_str: &str, quoted: bool) -> Fix {
    let name_start = ctx.source_offset(name);
    let attr_end = ctx.source_end(value_str) + usize::from(quoted);
    let fix_start = reverse_consume_ws(ctx.source().as_bytes(), name_start);
    Fix::safe_edit(Edit::deletion(span(fix_start, attr_end - fix_start)))
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

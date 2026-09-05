use miette::SourceSpan;

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
    let fix_start = ctx.source()[..name_start].trim_ascii_end().len();
    Fix::safe_edit(Edit::deletion(span(fix_start, attr_end - fix_start)))
}

/// An edit deleting a whole comment, taking the line with it when the comment has it to
/// itself, so no blank line is left behind.
pub fn delete_comment(ctx: &LintContext<'_>, comment: &str) -> Edit {
    let source = ctx.source();
    let start = ctx.source_offset(comment);
    let end = start + comment.len();
    let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = source[end..]
        .find('\n')
        .map_or(source.len(), |i| end + i + 1);
    let (before, after) = (&source[line_start..start], &source[end..line_end]);
    let (start, end) = if before.trim_ascii().is_empty() && after.trim_ascii().is_empty() {
        (line_start, line_end)
    } else {
        // Whitespace is absorbed within the line only, so the fix never joins two lines.
        (line_start + before.trim_ascii_end().len(), end)
    };
    Edit::deletion(span(start, end - start))
}

/// The span to report and the edit dropping `remove` from a directive's `codes`.
///
/// A single code is spanned and taken out with its separator; the comment goes once no code
/// would be left; several codes among others rewrite the list in place and span the comment.
///
/// # Panics
///
/// Panics if a code in `remove` is not listed in `codes`.
pub fn delete_codes_or_comment(
    ctx: &LintContext<'_>,
    comment: &str,
    codes: &[&str],
    remove: &[&str],
) -> (SourceSpan, Edit) {
    let start_of = |slice: &str| ctx.source_offset(slice);
    let end_of = |slice: &str| ctx.source_offset(slice) + slice.len();
    let span_of = |slice: &str| span(start_of(slice), slice.len());
    if let [only] = codes {
        return (span_of(only), delete_comment(ctx, comment));
    }
    if let [code] = remove {
        // Offsets come from the listed slice: `remove` may hold an equal string from elsewhere.
        let index = codes
            .iter()
            .position(|listed| listed == code)
            .expect("a code to remove is listed");
        let code = codes[index];
        let (start, end) = codes.get(index + 1).map_or_else(
            || (end_of(codes[index - 1]), end_of(code)),
            |next| (start_of(code), start_of(next)),
        );
        return (span_of(code), Edit::deletion(span(start, end - start)));
    }
    let mut remaining: Vec<&str> = Vec::new();
    for code in codes {
        if !remove.contains(code) && !remaining.contains(code) {
            remaining.push(code);
        }
    }
    let edit = if remaining.is_empty() {
        delete_comment(ctx, comment)
    } else {
        let (start, end) = (start_of(codes[0]), end_of(codes[codes.len() - 1]));
        Edit::replacement(remaining.join(", "), span(start, end - start))
    };
    (span_of(comment), edit)
}

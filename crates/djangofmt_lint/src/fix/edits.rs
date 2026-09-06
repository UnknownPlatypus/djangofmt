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

/// An edit deleting a comment, and its line when it has the line to itself.
///
/// ```diff
/// -{# djangofmt: ignore[gone-rule] #}
///  <form method="yes"></form>
/// ```
///
/// When sharing a line, only the comment goes. Surrounding whitespaces are kept.
///
/// ```diff
/// -<div> {# djangofmt: ignore[gone-rule] #} <p>hi</p></div>
/// +<div>  <p>hi</p></div>
/// ```
pub fn delete_comment(ctx: &LintContext<'_>, comment: &str) -> Edit {
    let source = ctx.source();
    let start = ctx.source_offset(comment);
    let end = start + comment.len();

    let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = source[end..]
        .find('\n')
        .map_or(source.len(), |i| end + i + 1);

    let standalone = source[line_start..start].trim_ascii().is_empty()
        && source[end..line_end].trim_ascii().is_empty();
    let (delete_start, delete_end) = if standalone {
        (line_start, line_end)
    } else {
        (start, end)
    };
    Edit::deletion(span(delete_start, delete_end - delete_start))
}

/// Drops `remove` from a directive's `codes`, returning the span to report and the edit.
///
/// When one code is dropped, only that entry and its comma are removed:
///
/// ```diff
/// -{# djangofmt: ignore[not-a-rule, invalid-attr-value] #}
/// +{# djangofmt: ignore[invalid-attr-value] #}
/// ```
///
/// When several codes go but some stay, the list is rewritten:
///
/// ```diff
/// -{# djangofmt: ignore[not-a-rule, invalid-attr-value, nor-this, empty-attr-value] #}
/// +{# djangofmt: ignore[invalid-attr-value, empty-attr-value] #}
/// ```
///
/// When nothing remains, the whole comment goes:
///
/// ```diff
/// -{# djangofmt: file-ignore[nonexistent-rule, also-not-a-rule] #}
///  <form method="yes"></form>
/// ```
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
    let mut listed = codes
        .iter()
        .enumerate()
        .filter(|(_, listed)| remove.contains(listed));
    if let (Some((index, code)), None) = (listed.next(), listed.next()) {
        let (start, end) = codes.get(index + 1).map_or_else(
            || (end_of(codes[index - 1]), end_of(code)),
            |next| (start_of(code), start_of(next)),
        );
        return (span_of(code), Edit::deletion(span(start, end - start)));
    }
    let remaining: Vec<&str> = codes
        .iter()
        .copied()
        .filter(|code| !remove.contains(code))
        .collect();
    let edit = if remaining.is_empty() {
        delete_comment(ctx, comment)
    } else {
        let (start, end) = (start_of(codes[0]), end_of(codes[codes.len() - 1]));
        Edit::replacement(remaining.join(", "), span(start, end - start))
    };
    (span_of(comment), edit)
}

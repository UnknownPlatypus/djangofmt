use std::borrow::Cow;

use markup_fmt::ast::Comment;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::{HTML_COMMENT, TEMPLATE_COMMENT, strip_bom};
use crate::suppression::{FILE_IGNORE, IGNORE, NAMESPACE, ReservedCode};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for the formatter's ignore directive written as an HTML comment.
///
/// ## Why is this bad?
/// `<!-- djangofmt:ignore -->` is the deprecated spelling of `{# djangofmt: ignore[format] #}`.
/// Prefer the django comment form to avoid sending the HTML comment to the client.
///
/// ## Example
/// ```html
/// <!-- djangofmt:ignore -->
/// <div   class="keep-this-unformatted"   >Content</div>
/// ```
///
/// Use instead:
/// ```html
/// {# djangofmt: ignore[format] #}
/// <div   class="keep-this-unformatted"   >Content</div>
/// ```
///
/// ## Fix safety
/// The fix is marked as unsafe when the comment lists a code beyond `format`: the linter reads
/// no HTML comment, so that code silences nothing today, and the `{# #}` rewrite would start
/// honoring it.
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct RedirectedIgnore {
    /// Whether the comment is the legacy whole-file opt-out, spelled `file-ignore[format]`.
    pub file_level: bool,
    /// Why the rewrite is left to the author, when it is.
    pub unfixable: Option<Unfixable>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unfixable {
    /// Django's `{# #}` comments are single-line.
    MultiLine,
    /// A `#}` in the body would end the new comment early.
    ClosesEarly,
}

impl Unfixable {
    /// Why the comment body cannot move into a `{# #}` comment as is, if it cannot.
    fn in_body(body: &str) -> Option<Self> {
        if body.contains('\n') {
            Some(Self::MultiLine)
        } else if body.contains(TEMPLATE_COMMENT.close) {
            Some(Self::ClosesEarly)
        } else {
            None
        }
    }
}

impl RedirectedIgnore {
    /// The `{# #}` comment the directive should be written as.
    const fn spelling(&self) -> &'static str {
        if self.file_level {
            "{# djangofmt: file-ignore[format] #}"
        } else {
            "{# djangofmt: ignore[format] #}"
        }
    }
}

impl Violation for RedirectedIgnore {
    const RULE: Rule = Rule::RedirectedIgnore;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        "Deprecated HTML ignore comment".into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        let spelling = self.spelling();
        Some(
            match self.unfixable {
                None => format!("Write it as `{spelling}` instead"),
                Some(Unfixable::MultiLine) => format!("Write it as `{spelling}` on a single line"),
                Some(Unfixable::ClosesEarly) => {
                    format!("Write it as `{spelling}`, without the `#}}` in its body")
                }
            }
            .into(),
        )
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some(if self.file_level {
            "Rewrite as `{# djangofmt: file-ignore[format] #}`"
        } else {
            "Rewrite as `{# djangofmt: ignore[format] #}`"
        })
    }
}

/// An ignore directive this rule has to read, which [`IgnoreDirective`] cannot: the bare
/// `djangofmt:ignore`, which carries no code for the linter to act on, yet is exactly the
/// deprecated spelling flagged here. It also keeps the trailing reason, so the fix can carry
/// it over to the `{# #}` rewrite.
///
/// Only what the formatter honors parses: the bare directive, or `ignore[...]` listing `format`.
///
/// [`IgnoreDirective`]: crate::suppression::IgnoreDirective
#[derive(Debug, PartialEq, Eq)]
struct FormatIgnoreDirective<'s> {
    /// The listed codes, none for the bare directive.
    codes: Vec<&'s str>,
    /// The free text trailing the directive, if any.
    reason: &'s str,
}

impl<'s> FormatIgnoreDirective<'s> {
    /// `None` unless `body` is one of the
    /// [`FORMAT_IGNORE_DIRECTIVES`](crate::FORMAT_IGNORE_DIRECTIVES).
    #[must_use]
    fn parse(body: &'s str) -> Option<Self> {
        let directive = markup_fmt::parse_directive(body, NAMESPACE, &[IGNORE])?.ok()?;
        let format = ReservedCode::Format.as_str();
        if !directive.codes.is_empty() && !directive.codes.contains(&format) {
            return None;
        }
        // The reason trails the code list, or the bare keyword. Nothing before either
        // holds a `]` or spells `ignore`, so the first match is the right one.
        let after = if directive.codes.is_empty() {
            &body[body.find(IGNORE)? + IGNORE.len()..]
        } else {
            &body[body.find(']')? + 1..]
        };
        // HTML comments may pad the body with dashes, and a `:` may introduce the reason.
        let reason = after
            .trim_matches(|c: char| c.is_whitespace() || c == '-')
            .trim_start_matches(':')
            .trim();
        Some(Self {
            codes: directive.codes,
            reason,
        })
    }

    /// Whether it is the bare `djangofmt:ignore`, the spelling of the legacy whole-file opt-out.
    #[must_use]
    const fn is_bare(&self) -> bool {
        self.codes.is_empty()
    }

    /// Whether the code list stops at `format`. The linter reads no HTML comment, so any other
    /// code silences nothing until a `{# #}` rewrite starts honoring it.
    #[must_use]
    fn is_format_only(&self) -> bool {
        self.codes
            .iter()
            .all(|code| *code == ReservedCode::Format.as_str())
    }

    /// The `{# #}` comment spelling this directive out: its codes, `format` for the bare
    /// directive, then its reason. `file_level` uses the `file-ignore` keyword.
    #[must_use]
    fn to_template_comment(&self, file_level: bool) -> String {
        let keyword = if file_level { FILE_IGNORE } else { IGNORE };
        let codes = if self.codes.is_empty() {
            ReservedCode::Format.as_str().to_owned()
        } else {
            self.codes.join(", ")
        };
        let reason = if self.reason.is_empty() {
            String::new()
        } else {
            format!(": {}", self.reason)
        };
        format!("{{# {NAMESPACE}: {keyword}[{codes}]{reason} #}}")
    }
}

pub fn check(comment: &Comment<'_>, checker: &Checker<'_>) {
    let Some(directive) = FormatIgnoreDirective::parse(comment.raw) else {
        return;
    };
    let html_comment = HTML_COMMENT.enclosing_comment(checker, comment.raw);
    let violation = RedirectedIgnore {
        file_level: is_legacy_file_opt_out(&directive, html_comment, checker),
        unfixable: Unfixable::in_body(comment.raw),
    };
    let span = checker.source_span(html_comment);
    let mut guard = checker.report_diagnostic(&violation, span);
    if violation.unfixable.is_none() {
        let rewritten = directive.to_template_comment(violation.file_level);
        let edit = Edit::replacement(rewritten, span);
        guard.set_fix(if directive.is_format_only() {
            Fix::safe_edit(edit)
        } else {
            Fix::unsafe_edit(edit)
        });
    }
}

/// Leading the file, the formatter's bare directive is its legacy whole-file opt-out.
/// Anything before it but a BOM, whitespace included, makes it guard the first node instead.
fn is_legacy_file_opt_out(
    directive: &FormatIgnoreDirective<'_>,
    html_comment: &str,
    checker: &Checker<'_>,
) -> bool {
    let before = &checker.context().source()[..checker.source_offset(html_comment)];
    directive.is_bare() && strip_bom(before).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suppression::FORMAT_IGNORE_DIRECTIVES;
    use rstest::rstest;

    /// The linter flags exactly the comments the formatter is configured to honor.
    #[rstest]
    fn format_ignore_directive_matches_the_formatter(
        #[values(
            " djangofmt:ignore ",
            "- djangofmt:ignore -",
            "djangofmt: ignore[format]",
            "djangofmt: ignore[format, a]: reason",
            "djangofmt: ignore[a]",
            "djangofmt: ignore[]",
            "djangofmt: file-ignore[format]",
            "See djangofmt: https://example.com"
        )]
        body: &str,
    ) {
        let formatter_honors = FORMAT_IGNORE_DIRECTIVES
            .iter()
            .any(|directive| markup_fmt::matches_directive(body, directive));
        assert_eq!(
            FormatIgnoreDirective::parse(body).is_some(),
            formatter_honors,
            "{body}"
        );
    }
}

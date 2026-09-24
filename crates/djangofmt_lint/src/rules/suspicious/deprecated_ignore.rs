use std::borrow::Cow;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::{CommentDelimiters, HTML_COMMENT, TEMPLATE_COMMENT, strip_bom};
use crate::suppression::{FILE_IGNORE, IGNORE, LEGACY_IGNORE_DIRECTIVE, NAMESPACE, ReservedCode};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for the formatter's legacy bare ignore directive, in either comment style.
///
/// ## Why is this bad?
/// `{# djangofmt:ignore #}` and `<!-- djangofmt:ignore -->` are the deprecated spellings of
/// `{# djangofmt: ignore[format] #}`, which names what it suppresses and shares its grammar
/// with every other suppression. The HTML form is also shipped to the client.
///
/// ## Example
/// ```html
/// {# djangofmt:ignore #}
/// <div   class="keep-this-unformatted"   >Content</div>
/// ```
///
/// Use instead:
/// ```html
/// {# djangofmt: ignore[format] #}
/// <div   class="keep-this-unformatted"   >Content</div>
/// ```
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "1.0.0")]
pub struct DeprecatedIgnore {
    /// Whether the directive sits in an HTML comment, the spelling also rendered to the client.
    pub in_html: bool,
    /// Whether the comment is the legacy whole-file opt-out, spelled `file-ignore[format]`.
    pub file_level: bool,
    /// Whether that opt-out quarantines a file that does not parse, spelled
    /// `file-ignore[invalid-syntax]` instead, so the file gets formatted once it parses.
    pub quarantines: bool,
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

impl DeprecatedIgnore {
    /// The `{# #}` comment the directive should be written as, `reason` carried over when any.
    fn rewrite(&self, reason: &str) -> String {
        let keyword = if self.file_level { FILE_IGNORE } else { IGNORE };
        let code = if self.quarantines {
            ReservedCode::InvalidSyntax
        } else {
            ReservedCode::Format
        };
        let reason = if reason.is_empty() {
            String::new()
        } else {
            format!(": {reason}")
        };
        format!("{{# {NAMESPACE}: {keyword}[{code}]{reason} #}}")
    }
}

impl Violation for DeprecatedIgnore {
    const RULE: Rule = Rule::DeprecatedIgnore;
    const CATEGORY: RuleCategory = RuleCategory::Suspicious;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        if self.in_html {
            "Deprecated HTML ignore comment".into()
        } else {
            "Deprecated ignore comment".into()
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        let spelling = self.rewrite("");
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
        Some(if self.quarantines {
            "Rewrite as `{# djangofmt: file-ignore[invalid-syntax] #}`"
        } else if self.file_level {
            "Rewrite as `{# djangofmt: file-ignore[format] #}`"
        } else {
            "Rewrite as `{# djangofmt: ignore[format] #}`"
        })
    }
}

/// The free text trailing the directive keyword, empty when it carries none.
fn reason(body: &str) -> &str {
    // The keyword is the only `ignore` the body can spell, so the first match is the right one.
    // HTML comments may pad the reason with dashes, and a `:` may introduce it.
    body.find(IGNORE).map_or("", |keyword| {
        body[keyword + IGNORE.len()..]
            .trim_matches(|c: char| c.is_whitespace() || c == '-')
            .trim_start_matches(':')
            .trim()
    })
}

/// Lint a comment of either style, `body` being its text between `delimiters`.
pub fn check(checker: &Checker<'_>, delimiters: CommentDelimiters, body: &str) {
    report(checker, delimiters, body, false);
}

/// Lint the comment leading a file that does not parse: with no AST, it is read from the raw
/// source, as [`FileIgnores::parse`](crate::FileIgnores::parse) reads it.
pub fn check_quarantined(checker: &Checker<'_>) {
    let source = strip_bom(checker.context().source());
    for delimiters in [TEMPLATE_COMMENT, HTML_COMMENT] {
        if let Some(body) = delimiters.body(source) {
            report(checker, delimiters, body, true);
        }
    }
}

/// Report `body` when it is the legacy directive, rewriting its whole comment as a coded
/// `{# #}` directive when it fits.
fn report(checker: &Checker<'_>, delimiters: CommentDelimiters, body: &str, quarantines: bool) {
    if !markup_fmt::matches_directive(body, LEGACY_IGNORE_DIRECTIVE) {
        return;
    }
    let comment = delimiters.enclosing_comment(checker, body);
    let violation = DeprecatedIgnore {
        in_html: delimiters == HTML_COMMENT,
        file_level: is_legacy_file_opt_out(checker, comment),
        quarantines,
        unfixable: Unfixable::in_body(body),
    };
    let span = checker.source_span(comment);
    let mut guard = checker.report_diagnostic(&violation, span);
    if violation.unfixable.is_none() {
        let rewritten = violation.rewrite(reason(body));
        guard.set_fix(Fix::safe_edit(Edit::replacement(rewritten, span)));
    }
}

/// Leading the file, the formatter's bare directive is its legacy whole-file opt-out.
fn is_legacy_file_opt_out(checker: &Checker<'_>, comment: &str) -> bool {
    let before = &checker.context().source()[..checker.source_offset(comment)];
    strip_bom(before).is_empty()
}

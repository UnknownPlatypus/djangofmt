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
    /// What the directive opts out, which picks its new spelling.
    pub scope: Scope,
    /// Why the rewrite is left to the author, when it is.
    pub unfixable: Option<Unfixable>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The next node, spelled `ignore[format]`.
    Node,
    /// The whole file, spelled `file-ignore[format]`.
    File,
    /// A whole file that does not parse,
    /// spelled `file-ignore[invalid-syntax]` so it gets formatted once it parses.
    Quarantine,
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
        let (keyword, code) = match self.scope {
            Scope::Node => (IGNORE, ReservedCode::Format),
            Scope::File => (FILE_IGNORE, ReservedCode::Format),
            Scope::Quarantine => (FILE_IGNORE, ReservedCode::InvalidSyntax),
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
        Some(match self.scope {
            Scope::Node => "Rewrite as `{# djangofmt: ignore[format] #}`",
            Scope::File => "Rewrite as `{# djangofmt: file-ignore[format] #}`",
            Scope::Quarantine => "Rewrite as `{# djangofmt: file-ignore[invalid-syntax] #}`",
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
    if markup_fmt::matches_directive(body, LEGACY_IGNORE_DIRECTIVE) {
        let whole_comment = delimiters.enclosing_comment(checker, body);
        report(checker, whole_comment, body, delimiters == HTML_COMMENT);
    }
}

/// Report the whole comment and rewrite it as a coded `{# #}` directive when it fits.
fn report(checker: &Checker<'_>, comment: &str, comment_body: &str, in_html: bool) {
    let violation = DeprecatedIgnore {
        in_html,
        scope: scope(checker, comment),
        unfixable: Unfixable::in_body(comment_body),
    };
    let span = checker.source_span(comment);
    let mut guard = checker.report_diagnostic(&violation, span);
    if violation.unfixable.is_none() {
        let rewritten = violation.rewrite(reason(comment_body));
        guard.set_fix(Fix::safe_edit(Edit::replacement(rewritten, span)));
    }
}

/// What the directive opts out: leading the file, it is the formatter's legacy whole-file opt-out.
fn scope(checker: &Checker<'_>, comment: &str) -> Scope {
    let before = &checker.context().source()[..checker.source_offset(comment)];
    if !strip_bom(before).is_empty() {
        Scope::Node
    } else if checker.context().is_quarantined() {
        Scope::Quarantine
    } else {
        Scope::File
    }
}

//! File-wide opt-outs declared by `{# djangofmt: file-ignore[...] #}` comments.
//!
//! Rules are always listed explicitly, and only `{# #}` comments carry
//! directives: HTML comments survive in rendered output.

/// `file-ignore[...]` code suppressing parse errors.
pub const INVALID_SYNTAX: &str = "invalid-syntax";

/// `file-ignore[...]` code skipping the formatter.
pub const FORMAT: &str = "format";

/// A parsed suppression directive.
#[derive(Debug, PartialEq, Eq)]
enum Directive<'s> {
    /// `djangofmt: ignore[...]` — suppress on the following node.
    Ignore(Vec<&'s str>),
    /// `djangofmt: file-ignore[...]` — suppress for the whole file.
    FileIgnore(Vec<&'s str>),
}

/// A comment body stripped of Jinja's whitespace-control markers (`{#- ... -#}`),
/// which are part of the delimiter rather than of the directive.
fn directive_body(raw: &str) -> &str {
    raw.trim()
        .trim_start_matches(['-', '+'])
        .trim_end_matches('-')
        .trim()
}

/// Parse a comment body into a suppression directive.
///
/// Grammar: `djangofmt:` (whitespace allowed around the colon), then
/// `ignore[...]` or `file-ignore[...]` with a non-empty comma-separated rule
/// list and nothing after the closing bracket.
fn parse_directive(raw: &str) -> Option<Directive<'_>> {
    let rest = directive_body(raw)
        .strip_prefix("djangofmt")?
        .trim_start()
        .strip_prefix(':')?
        .trim_start();
    let (file_level, rest) = match rest.strip_prefix("file-ignore[") {
        Some(rest) => (true, rest),
        None => (false, rest.strip_prefix("ignore[")?),
    };
    let codes: Vec<&str> = rest
        .strip_suffix(']')?
        .split(',')
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .collect();
    if codes.is_empty() {
        return None;
    }
    Some(if file_level {
        Directive::FileIgnore(codes)
    } else {
        Directive::Ignore(codes)
    })
}

/// File-wide opt-outs declared by the leading comment of a file.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FileIgnores {
    /// The formatter skips the whole file (`file-ignore[format]`).
    pub format: bool,
    /// Parse errors are suppressed and the file skipped (`file-ignore[invalid-syntax]`).
    pub invalid_syntax: bool,
}

/// Opt-outs from the file's leading comment, read straight from the raw source
/// so they can be honored even when the file fails to parse.
///
/// The bare legacy `djangofmt:ignore` (in either comment style) predates rule
/// codes and opted the file out of everything: it maps to both flags, but only
/// at the very start of the file since it also serves as a node-level directive.
#[must_use]
pub fn file_ignores(source: &str) -> FileIgnores {
    // A UTF-8 BOM is not Rust whitespace, so strip it explicitly.
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);

    // The bare legacy directive doubles as a node-level formatter directive,
    // so it is only file-level when nothing (not even whitespace) precedes it.
    let legacy_body =
        leading_comment(source, "{#", "#}").or_else(|| leading_comment(source, "<!--", "-->"));
    if let Some(body) = legacy_body
        && markup_fmt::starts_with_directive(directive_body(body), "djangofmt:ignore")
    {
        return FileIgnores {
            format: true,
            invalid_syntax: true,
        };
    }
    // `file-ignore[...]` is unambiguously file-level: leading whitespace is fine.
    match leading_comment(source.trim_start(), "{#", "#}").map(parse_directive) {
        Some(Some(Directive::FileIgnore(codes))) => FileIgnores {
            format: codes.contains(&FORMAT),
            invalid_syntax: codes.contains(&INVALID_SYNTAX),
        },
        _ => FileIgnores::default(),
    }
}

/// The body of a leading `open`..`close` comment, if the text starts with one.
fn leading_comment<'s>(text: &'s str, open: &str, close: &str) -> Option<&'s str> {
    let body = text.strip_prefix(open)?;
    Some(&body[..body.find(close)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_directives() {
        assert_eq!(
            parse_directive(" djangofmt: ignore[a, b ,c] "),
            Some(Directive::Ignore(vec!["a", "b", "c"]))
        );
        assert_eq!(
            parse_directive("djangofmt:file-ignore[invalid-syntax]"),
            Some(Directive::FileIgnore(vec!["invalid-syntax"]))
        );
        assert_eq!(
            parse_directive("djangofmt : file-ignore[invalid-syntax]"),
            Some(Directive::FileIgnore(vec!["invalid-syntax"]))
        );
    }

    #[test]
    fn reject_non_directives() {
        assert_eq!(parse_directive(" djangofmt:ignore "), None); // formatter directive
        assert_eq!(parse_directive("djangofmt: ignore[]"), None); // explicit rules only
        assert_eq!(parse_directive("djangofmt: ignore[ , ]"), None); // only separators
        assert_eq!(parse_directive("djangofmt: ignore[a] trailing"), None); // nothing after bracket
        assert_eq!(parse_directive("ignore[a]"), None); // must start with djangofmt:
    }

    #[test]
    fn detect_file_level_opt_outs() {
        let all = FileIgnores {
            format: true,
            invalid_syntax: true,
        };
        let syntax_only = FileIgnores {
            format: false,
            invalid_syntax: true,
        };
        let format_only = FileIgnores {
            format: true,
            invalid_syntax: false,
        };

        assert_eq!(
            file_ignores("{# djangofmt: file-ignore[invalid-syntax] #}\n<div id=>"),
            syntax_only
        );
        assert_eq!(
            file_ignores("{# djangofmt: file-ignore[format] #}\n<div></div>"),
            format_only
        );
        assert_eq!(
            file_ignores("{# djangofmt: file-ignore[format, invalid-syntax] #}"),
            all
        );
        // Jinja whitespace-control markers are part of the delimiter.
        assert_eq!(
            file_ignores("{#- djangofmt: file-ignore[format] -#}"),
            format_only
        );
        // A UTF-8 BOM or leading whitespace before the directive is tolerated.
        assert_eq!(
            file_ignores("\u{feff}{# djangofmt: file-ignore[invalid-syntax] #}\n<div id=>"),
            syntax_only
        );
        assert_eq!(
            file_ignores("\n  {# djangofmt: file-ignore[foo, invalid-syntax] #}\n<div id=>"),
            syntax_only
        );

        // The bare legacy directive opts out of everything, in both styles,
        // with whitespace tolerated around the colon.
        assert_eq!(file_ignores("{# djangofmt:ignore #}\n<div id=>"), all);
        assert_eq!(file_ignores("<!-- djangofmt:ignore -->\n<div id=>"), all);
        assert_eq!(file_ignores("{# djangofmt : ignore #}\n<div id=>"), all);
        assert_eq!(
            file_ignores("\u{feff}{# djangofmt:ignore #}\n<div id=>"),
            all
        );
        // Preceded by whitespace, the bare directive is node-level, not file-level.
        assert_eq!(
            file_ignores("\n  {# djangofmt:ignore #}\n<div id=>"),
            FileIgnores::default()
        );
        assert_eq!(
            file_ignores(" <!-- djangofmt:ignore -->\n<div id=>"),
            FileIgnores::default()
        );

        // Bracketed directives only count in `{# #}` comments.
        assert_eq!(
            file_ignores("<!-- djangofmt: file-ignore[invalid-syntax] -->\n<div id=>"),
            FileIgnores::default()
        );
        // Lint codes, node-level directives and plain markup are not opt-outs.
        assert_eq!(
            file_ignores("{# djangofmt: file-ignore[missing-img-alt] #}\n<div id=>"),
            FileIgnores::default()
        );
        assert_eq!(
            file_ignores("{# djangofmt: ignore[invalid-syntax] #}\n<div id=>"),
            FileIgnores::default()
        );
        assert_eq!(file_ignores("<div id=>"), FileIgnores::default());
    }
}

//! File-wide opt-outs declared by `{# djangofmt: file-ignore[...] #}` comments.

use std::str::FromStr;

/// A code accepted in `file-ignore[...]`; unknown codes are ignored.
#[derive(Debug, PartialEq, Eq, strum::EnumString)]
#[strum(serialize_all = "kebab-case")]
enum FileIgnoreCode {
    /// Suppress parse errors: both commands skip the file.
    InvalidSyntax,
    /// Skip the formatter.
    Format,
}

/// A parsed suppression directive.
#[derive(Debug, PartialEq, Eq)]
enum Directive<'s> {
    /// `djangofmt: ignore[...]` suppress on the following node.
    Ignore(Vec<&'s str>),
    /// `djangofmt: file-ignore[...]` suppress for the whole file.
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
/// Grammar: `djangofmt:` (whitespace allowed around the colon),
/// then `ignore[...]` or `file-ignore[...]` with a non-empty comma-separated rule list.
/// Anything after the closing bracket is a free-text reason, ignored.
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
        .split_once(']')?
        .0
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
        Some(Some(Directive::FileIgnore(codes))) => {
            let codes: Vec<_> = codes
                .iter()
                .filter_map(|code| FileIgnoreCode::from_str(code).ok())
                .collect();
            FileIgnores {
                format: codes.contains(&FileIgnoreCode::Format),
                invalid_syntax: codes.contains(&FileIgnoreCode::InvalidSyntax),
            }
        }
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
    use rstest::rstest;

    const ALL: FileIgnores = FileIgnores {
        format: true,
        invalid_syntax: true,
    };
    const SYNTAX_ONLY: FileIgnores = FileIgnores {
        format: false,
        invalid_syntax: true,
    };
    const FORMAT_ONLY: FileIgnores = FileIgnores {
        format: true,
        invalid_syntax: false,
    };
    const NONE: FileIgnores = FileIgnores {
        format: false,
        invalid_syntax: false,
    };

    #[rstest]
    #[case::node(" djangofmt: ignore[a, b ,c] ", Directive::Ignore(vec!["a", "b", "c"]))]
    #[case::file("djangofmt:file-ignore[invalid-syntax]", Directive::FileIgnore(vec!["invalid-syntax"]))]
    #[case::spaced_colon("djangofmt : file-ignore[invalid-syntax]", Directive::FileIgnore(vec!["invalid-syntax"]))]
    #[case::reason("djangofmt: ignore[a]: free-text reason", Directive::Ignore(vec!["a"]))]
    fn parse_directives(#[case] comment: &str, #[case] expected: Directive) {
        assert_eq!(parse_directive(comment), Some(expected));
    }

    #[rstest]
    fn reject_non_directives(
        #[values(
            " djangofmt:ignore ",            // formatter directive
            "djangofmt: ignore[]",           // explicit rules only
            "djangofmt: ignore[ , ]", // only separators
            "ignore[a]"               // must start with djangofmt:
        )]
        comment: &str,
    ) {
        assert_eq!(parse_directive(comment), None);
    }

    #[rstest]
    #[case::invalid_syntax("{# djangofmt: file-ignore[invalid-syntax] #}\n<div id=>", SYNTAX_ONLY)]
    #[case::format("{# djangofmt: file-ignore[format] #}\n<div></div>", FORMAT_ONLY)]
    #[case::both_codes("{# djangofmt: file-ignore[format, invalid-syntax] #}", ALL)]
    // Jinja whitespace-control markers are part of the delimiter.
    #[case::whitespace_control("{#- djangofmt: file-ignore[format] -#}", FORMAT_ONLY)]
    // Anything after the closing bracket is a free-text reason.
    #[case::reason("{# djangofmt: file-ignore[format]: vendored file #}", FORMAT_ONLY)]
    // A UTF-8 BOM or leading whitespace before the directive is tolerated.
    #[case::bom(
        "\u{feff}{# djangofmt: file-ignore[invalid-syntax] #}\n<div id=>",
        SYNTAX_ONLY
    )]
    #[case::leading_whitespace(
        "\n  {# djangofmt: file-ignore[foo, invalid-syntax] #}\n<div id=>",
        SYNTAX_ONLY
    )]
    // The bare legacy directive opts out of everything, in both styles,
    // with whitespace tolerated around the colon.
    #[case::legacy_jinja("{# djangofmt:ignore #}\n<div id=>", ALL)]
    #[case::legacy_html("<!-- djangofmt:ignore -->\n<div id=>", ALL)]
    #[case::legacy_spaced_colon("{# djangofmt : ignore #}\n<div id=>", ALL)]
    #[case::legacy_bom("\u{feff}{# djangofmt:ignore #}\n<div id=>", ALL)]
    // Preceded by whitespace, the bare directive is node-level, not file-level.
    #[case::legacy_after_newline("\n  {# djangofmt:ignore #}\n<div id=>", NONE)]
    #[case::legacy_after_space(" <!-- djangofmt:ignore -->\n<div id=>", NONE)]
    // Bracketed directives only count in `{# #}` comments.
    #[case::html_comment("<!-- djangofmt: file-ignore[invalid-syntax] -->\n<div id=>", NONE)]
    // Lint codes, node-level directives and plain markup are not opt-outs.
    #[case::lint_code("{# djangofmt: file-ignore[missing-img-alt] #}\n<div id=>", NONE)]
    #[case::node_level("{# djangofmt: ignore[invalid-syntax] #}\n<div id=>", NONE)]
    #[case::plain_markup("<div id=>", NONE)]
    fn detect_file_level_opt_outs(#[case] source: &str, #[case] expected: FileIgnores) {
        assert_eq!(file_ignores(source), expected);
    }
}

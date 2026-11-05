//! `{# djangofmt: ignore[...] #}` and `{# djangofmt: file-ignore[...] #}` directives.
use std::str::FromStr;

use markup_fmt::ParseErrorKind;

use crate::comment::{HTML_COMMENT, TEMPLATE_COMMENT, strip_bom};

/// An ignore code naming no rule: it opts out of a whole stage rather than one lint.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    strum::EnumString,
    strum::Display,
    strum::IntoStaticStr,
    strum::VariantNames,
)]
#[strum(serialize_all = "kebab-case")]
pub enum ReservedCode {
    /// Opts the node or file out of the formatter.
    Format,
    /// Suppresses parse errors; only `file-ignore[...]` may carry it.
    InvalidSyntax,
}

impl ReservedCode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        self.into()
    }
}

pub const NAMESPACE: &str = "djangofmt";
pub const IGNORE: &str = "ignore";
pub const FILE_IGNORE: &str = "file-ignore";

/// The formatter's legacy ignore directive.
pub const LEGACY_IGNORE_DIRECTIVE: &str = "djangofmt:ignore";

/// The formatter's canonical ignore directive, using unified error codes.
pub const IGNORE_DIRECTIVE: &str = "djangofmt:ignore[format]";

pub const FORMAT_IGNORE_DIRECTIVES: [&str; 2] = [LEGACY_IGNORE_DIRECTIVE, IGNORE_DIRECTIVE];

/// What an ignore comment asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum IgnoreDirective<'s> {
    /// `ignore[...]`, guarding the following node.
    Ignore(Vec<&'s str>),
    /// `file-ignore[...]`, covering the whole file.
    FileIgnore(Vec<&'s str>),
    /// Addressed to djangofmt, yet neither of the above nor the formatter's bare `ignore`.
    Malformed(ParseErrorKind),
}

impl<'s> IgnoreDirective<'s> {
    /// Parse a comment body with the grammar the formatter uses.
    ///
    /// `None` is a comment not addressed to djangofmt at all, or the formatter's bare `ignore`,
    /// which lists no code and is `deprecated-ignore`'s to report.
    #[must_use]
    pub fn parse(comment_body: &'s str) -> Option<Self> {
        let directive =
            match markup_fmt::parse_directive(comment_body, NAMESPACE, &[IGNORE, FILE_IGNORE])? {
                Ok(directive) => directive,
                Err(error) => return Some(Self::Malformed(error)),
            };
        Some(match (directive.keyword, directive.codes) {
            (IGNORE, codes) if codes.is_empty() => return None,
            (IGNORE, codes) => Self::Ignore(codes),
            (_, codes) if codes.is_empty() => Self::Malformed(ParseErrorKind::MissingCodes),
            (_, codes) => Self::FileIgnore(codes),
        })
    }

    /// The codes listed, none for a malformed directive.
    #[must_use]
    pub fn codes(&self) -> &[&'s str] {
        match self {
            Self::Ignore(codes) | Self::FileIgnore(codes) => codes,
            Self::Malformed(_) => &[],
        }
    }
}

/// File-wide opt-outs declared by the leading comment of a file: `file-ignore[...]` codes,
/// or the legacy bare `djangofmt:ignore` in `{# #}` or `<!-- -->` form, which sets both.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FileIgnores {
    /// The formatter skips the whole file (`file-ignore[format]`).
    pub format: bool,
    /// Parse errors are suppressed and the file skipped (`file-ignore[invalid-syntax]`).
    pub invalid_syntax: bool,
}

impl FileIgnores {
    /// Opt-outs from the file's leading comment, read straight from the raw source
    /// so they can be honored even when the file fails to parse.
    #[must_use]
    pub fn parse(source: &str) -> Self {
        Self::parse_head(source)
            .map(|(ignores, _)| ignores)
            .unwrap_or_default()
    }

    /// The source up to the end of its leading comment, when that comment quarantines the file,
    /// which is all the linter reads of a file that does not parse.
    #[must_use]
    pub fn quarantine_head(source: &str) -> Option<&str> {
        let (ignores, head) = Self::parse_head(source)?;
        ignores.invalid_syntax.then_some(head)
    }

    /// Opt-outs from the file's leading comment, a BOM and whitespace before it tolerated,
    /// with the source up to the end of that comment.
    fn parse_head(source: &str) -> Option<(Self, &str)> {
        let unmarked = strip_bom(source);
        let rest = unmarked.trim_start();
        let (delimiters, (body, after)) = [TEMPLATE_COMMENT, HTML_COMMENT]
            .into_iter()
            .find_map(|delimiters| Some((delimiters, delimiters.split(rest)?)))?;
        // The formatter's bare `ignore` doubles as its node-level directive,
        // so it is only file-level when nothing (not even whitespace) precedes it.
        let ignores = if markup_fmt::matches_directive(body, LEGACY_IGNORE_DIRECTIVE) {
            let leads = rest.len() == unmarked.len();
            Self {
                format: leads,
                invalid_syntax: leads,
            }
        } else if delimiters == TEMPLATE_COMMENT
            && let Some(IgnoreDirective::FileIgnore(codes)) = IgnoreDirective::parse(body)
        {
            codes.iter().fold(Self::default(), |mut ignores, code| {
                match ReservedCode::from_str(code) {
                    Ok(ReservedCode::Format) => ignores.format = true,
                    Ok(ReservedCode::InvalidSyntax) => ignores.invalid_syntax = true,
                    Err(_) => {}
                }
                ignores
            })
        } else {
            Self::default()
        };
        Some((ignores, &source[..source.len() - after.len()]))
    }
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

    /// The formatter matches the spelled-out directives; the linter parses namespace and keyword.
    #[test]
    fn ignore_directives_spell_the_namespace_and_keyword() {
        assert_eq!(LEGACY_IGNORE_DIRECTIVE, format!("{NAMESPACE}:{IGNORE}"));
        assert_eq!(
            IGNORE_DIRECTIVE,
            format!("{LEGACY_IGNORE_DIRECTIVE}[{}]", ReservedCode::Format)
        );
    }

    #[rstest]
    #[case::node(" djangofmt: ignore[a, b ,c] ", IgnoreDirective::Ignore(vec!["a", "b", "c"]))]
    #[case::file("djangofmt:file-ignore[invalid-syntax]", IgnoreDirective::FileIgnore(vec!["invalid-syntax"]))]
    #[case::spaced_colon("djangofmt : file-ignore[a]", IgnoreDirective::FileIgnore(vec!["a"]))]
    #[case::spaced_list("djangofmt: file-ignore [a]", IgnoreDirective::FileIgnore(vec!["a"]))]
    #[case::spaced_node_list("djangofmt: ignore [a]", IgnoreDirective::Ignore(vec!["a"]))]
    #[case::trailing_comma("djangofmt: ignore[a,]", IgnoreDirective::Ignore(vec!["a"]))]
    #[case::reason("djangofmt: ignore[a]: free-text reason", IgnoreDirective::Ignore(vec!["a"]))]
    #[case::whitespace_control("- djangofmt: ignore[a] -", IgnoreDirective::Ignore(vec!["a"]))]
    #[case::plus_whitespace_control("+ djangofmt: ignore[a]", IgnoreDirective::Ignore(vec!["a"]))]
    // A comment may wrap over lines, and a code may carry a `:` so `lint:code` reads as one.
    #[case::multiline("\n  djangofmt: ignore[a,\n  b]\n", IgnoreDirective::Ignore(vec!["a", "b"]))]
    #[case::namespaced_code("djangofmt: ignore[lint:a]", IgnoreDirective::Ignore(vec!["lint:a"]))]
    #[case::unknown_keyword(
        "djangofmt: silence[a]",
        IgnoreDirective::Malformed(ParseErrorKind::UnknownKeyword)
    )]
    #[case::bare_file_ignore(
        "djangofmt: file-ignore",
        IgnoreDirective::Malformed(ParseErrorKind::MissingCodes)
    )]
    #[case::empty_list(
        "djangofmt: ignore[]",
        IgnoreDirective::Malformed(ParseErrorKind::MissingCodes)
    )]
    #[case::unclosed_list(
        "djangofmt: ignore[a",
        IgnoreDirective::Malformed(ParseErrorKind::MissingBracket)
    )]
    #[case::missing_comma(
        "djangofmt: ignore[a b]",
        IgnoreDirective::Malformed(ParseErrorKind::MissingComma)
    )]
    #[case::only_separators(
        "djangofmt: ignore[ , ]",
        IgnoreDirective::Malformed(ParseErrorKind::InvalidCode)
    )]
    #[case::numeric_code(
        "djangofmt: ignore[1x]",
        IgnoreDirective::Malformed(ParseErrorKind::InvalidCode)
    )]
    fn parse_directives(
        #[case] comment_body: &'static str,
        #[case] expected: IgnoreDirective<'static>,
    ) {
        assert_eq!(IgnoreDirective::parse(comment_body), Some(expected));
    }

    #[rstest]
    fn skip_comments_the_linter_has_no_say_on(
        #[values(
            " djangofmt:ignore ",                 // the formatter's bare directive
            "djangofmt:ignore this is generated", // with a reason
            "djangofmt ignore[a]",                // missing colon
            "djangofmt-lint: ignore[a]",          // a namespace that merely starts the same
            "ignore[a]",                          // not addressed to djangofmt
            "See djangofmt: https://example.com"  // merely mentioning it
        )]
        comment_body: &str,
    ) {
        assert_eq!(IgnoreDirective::parse(comment_body), None);
    }

    #[rstest]
    #[case::invalid_syntax("{# djangofmt: file-ignore[invalid-syntax] #}\n<div id=>", SYNTAX_ONLY)]
    #[case::format("{# djangofmt: file-ignore[format] #}\n<div></div>", FORMAT_ONLY)]
    #[case::both_codes("{# djangofmt: file-ignore[format, invalid-syntax] #}", ALL)]
    // Jinja whitespace-control markers are part of the delimiter.
    #[case::whitespace_control("{#- djangofmt: file-ignore[format] -#}", FORMAT_ONLY)]
    // Anything after the closing bracket is a free-text reason.
    #[case::reason("{# djangofmt: file-ignore[format]: vendored file #}", FORMAT_ONLY)]
    // A UTF-8 BOM or leading whitespace before the comment is tolerated.
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
    // Bracketed ignore comments only count in `{# #}` comments.
    #[case::html_comment("<!-- djangofmt: file-ignore[invalid-syntax] -->\n<div id=>", NONE)]
    // Lint codes, node-level ignore comments and plain markup are not opt-outs.
    #[case::lint_code("{# djangofmt: file-ignore[missing-img-alt] #}\n<div id=>", NONE)]
    #[case::node_level("{# djangofmt: ignore[invalid-syntax] #}\n<div id=>", NONE)]
    #[case::plain_markup("<div id=>", NONE)]
    fn detect_file_level_opt_outs(#[case] source: &str, #[case] expected: FileIgnores) {
        assert_eq!(FileIgnores::parse(source), expected);
    }
}

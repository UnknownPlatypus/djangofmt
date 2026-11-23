//! The tokens Django's or Jinja's lexer reads before any HTML,
//! including the tags the HTML parser keeps as text, such as in attribute values or `<script>` bodies.

use std::sync::LazyLock;

use markup_fmt::Language;
use markup_fmt::ast::JinjaTag;
use memchr::{memchr, memchr2_iter, memchr3};
use regex::Regex;

use crate::dtl::{self, is_space};

/// A `{% %}` or `{{ }}` token the template engine compiles.
#[derive(Debug)]
pub enum Token<'s> {
    Tag { tag: JinjaTag<'s>, name: &'s str },
    Var(&'s str),
}

/// Lex `source` like Django's `Lexer`, or like Jinja's for any other language,
/// dropping comments and the bodies of `{% verbatim %}`, `{% comment %}` and Jinja's `{% raw %}`.
#[must_use]
pub fn lex(source: &str, language: Language) -> Vec<Token<'_>> {
    // Jinja's `raw_begin` and `raw_end`, where Python's `\s` also matches `\x1C` to `\x1F`.
    static RAW_BEGIN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\A\{%[-+]?[\s\x1C-\x1F]*raw[\s\x1C-\x1F]*-?%\}\z").expect("valid pattern")
    });
    static RAW_END: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\{%[-+]?[\s\x1C-\x1F]*endraw[\s\x1C-\x1F]*[-+]?%\}").expect("valid pattern")
    });
    let django = language == Language::Django;
    let bytes = source.as_bytes();
    let (mut tokens, mut pos) = (Vec::new(), 0);
    // Per token kind, the offset before which its closer is known missing, keeping the scan linear.
    let mut missing = [0; 3];
    // Django's lexer skips a verbatim body up to `end` + its opener, its parser a comment body.
    let (mut verbatim, mut in_comment) = (None, false);
    while let Some(i) = memchr(b'{', &bytes[pos..]) {
        let open = pos + i;
        // Like Django's regex, a failed match retries from the next byte.
        pos = open + 1;
        let (kind, close) = match bytes.get(pos) {
            Some(b'%') => (0, b'%'),
            Some(b'{') => (1, b'}'),
            Some(b'#') => (2, b'#'),
            _ => continue,
        };
        if open < missing[kind] {
            continue;
        }
        let body = open + 2;
        let found = if !django && close != b'#' {
            jinja_closer(&bytes[body..], close)
        } else {
            closer(&bytes[body..], close, django)
        };
        let len = match found {
            Ok(len) => len,
            Err(end) => {
                missing[kind] = body + end;
                continue;
            }
        };
        pos = body + len + 2;
        let content = &source[body..body + len];
        if close != b'%' {
            if close == b'}' && verbatim.is_none() && !in_comment {
                tokens.push(Token::Var(content));
            }
            continue;
        }
        let tag = JinjaTag {
            content,
            start: body,
        };
        let name = tag_name(&tag, language);
        if django && (verbatim.is_some() || in_comment || matches!(name, "verbatim" | "comment")) {
            let stripped = content.trim_matches(is_space);
            if let Some(opener) = verbatim {
                if stripped.strip_prefix("end") != Some(opener) {
                    continue;
                }
                verbatim = None;
            } else if stripped == "verbatim" || stripped.starts_with("verbatim ") {
                verbatim = Some(stripped);
            }
            if in_comment && stripped != "endcomment" {
                continue;
            }
            in_comment = name == "comment";
        }
        tokens.push(Token::Tag { tag, name });
        // Jinja reads a raw body as text up to the `{% endraw %}` it searches for, lexed next.
        if !django && name == "raw" && RAW_BEGIN.is_match(&source[open..pos]) {
            pos = RAW_END
                .find_at(source, pos)
                .map_or(source.len(), |end| end.start());
        }
    }
    tokens
}

/// The offset of the first `close` byte followed by `}`,
/// or where the search gave up, at a newline for Django.
fn closer(rest: &[u8], close: u8, django: bool) -> Result<usize, usize> {
    for i in memchr2_iter(b'}', if django { b'\n' } else { b'}' }, rest) {
        if rest[i] == b'\n' {
            return Err(i);
        }
        if i > 0 && rest[i - 1] == close {
            return Ok(i - 1);
        }
    }
    Err(rest.len())
}

/// The offset of the first `close` byte followed by `}` outside string literals and `{ }` pairs,
/// which Jinja lexes in a `{% %}` or `{{ }}`.
fn jinja_closer(rest: &[u8], close: u8) -> Result<usize, usize> {
    let first = closer(rest, close, false)?;
    // Most tokens hold no string or dict, so their first closer ends them.
    if memchr3(b'"', b'\'', b'{', &rest[..first]).is_none() {
        return Ok(first);
    }
    let (mut quote, mut depth, mut i) = (None, 0usize, 0);
    while i < rest.len() {
        match (quote, rest[i]) {
            (Some(_), b'\\') => i += 1,
            (Some(open), b) if b == open => quote = None,
            (None, b @ (b'"' | b'\'')) => quote = Some(b),
            (None, b) if b == close && depth == 0 && rest.get(i + 1) == Some(&b'}') => {
                return Ok(i);
            }
            (None, b'{') => depth += 1,
            (None, b'}') => depth = depth.saturating_sub(1),
            _ => {}
        }
        i += 1;
    }
    Err(rest.len())
}

/// Django names a tag by its first bit; Jinja strips whitespace-control markers first.
#[must_use]
pub fn tag_name<'s>(tag: &JinjaTag<'s>, language: Language) -> &'s str {
    if language == Language::Django {
        dtl::tag_name(tag.content)
    } else {
        // Unlike the fork's `parse_jinja_tag_name`, skips Python-only whitespace such as `\x1C`.
        let rest = tag
            .content
            .trim_start_matches(['+', '-'])
            .trim_start_matches(is_space);
        let end = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        &rest[..end]
    }
}

/// The bit after the tag name, such as a block's name, past Jinja's whitespace-control markers.
#[must_use]
pub fn tag_argument<'s>(tag: &JinjaTag<'s>, language: Language) -> Option<&'s str> {
    // Django has no whitespace-control markers, so a `-` there is part of a bit.
    let content = if language == Language::Django {
        tag.content
    } else {
        tag.content.trim_matches(['+', '-'])
    };
    dtl::bits(content).into_iter().nth(1)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn lexed(text: &str, language: Language) -> Vec<(&str, usize)> {
        lex(text, language)
            .iter()
            .map(|token| match token {
                Token::Tag { tag, .. } => (tag.content, tag.start),
                Token::Var(expr) => (*expr, expr.as_ptr() as usize - text.as_ptr() as usize),
            })
            .collect()
    }

    #[rstest]
    #[case::first_closer_wins(r#"{% x "{%}" %}"#, &[(r#" x "{"#, 2)])]
    #[case::variable_kept_comment_dropped("{{ '{% a %}' }} {# {% b %} #}", &[(" '{% a %}' ", 2)])]
    #[case::newline_ends_token("{% abc\n{% b %}", &[(" b ", 9)])]
    #[case::unclosed_variable_retries("{{% a %}", &[(" a ", 3)])]
    #[case::empty_token("{{}}", &[("", 2)])]
    #[case::verbatim_body_dropped(
        "{% verbatim x %}{% a %}{% endverbatim %}{% endverbatim x %}{% b %}",
        &[(" verbatim x ", 2), (" endverbatim x ", 42), (" b ", 61)]
    )]
    #[case::comment_body_dropped(
        r#"{% comment "note" %}{% a %}{{ c }}{% endcomment %}{% b %}"#,
        &[(r#" comment "note" "#, 2), (" endcomment ", 36), (" b ", 52)]
    )]
    #[case::raw_is_no_body("{% raw %}{% a %}{% endraw %}", &[(" raw ", 2), (" a ", 11), (" endraw ", 18)])]
    #[case::closer_compared_whole(
        "{% comment %}{%- endcomment %}{% a %}{% endcomment %}{% b %}",
        &[(" comment ", 2), (" endcomment ", 39), (" b ", 55)]
    )]
    fn django_tokens(#[case] text: &str, #[case] expected: &[(&str, usize)]) {
        assert_eq!(lexed(text, Language::Django), expected);
    }

    #[rstest]
    #[case::tag_spans_lines_and_strings(
        "{% set a =\n\"x%}\" ~ '\\'%}' ~ 5 % 2 %}",
        &[(" set a =\n\"x%}\" ~ '\\'%}' ~ 5 % 2 ", 2)]
    )]
    #[case::unclosed_string_leaves_the_tag_open("{% a '%}", &[])]
    #[case::raw_body_is_text(
        "{%- raw -%}{% b %}{{ c }}{%{% endraw %}{% d %}",
        &[("- raw -", 2), (" endraw ", 29), (" d ", 41)]
    )]
    #[case::unclosed_raw_hides_the_rest("{% raw %}{% a %}", &[(" raw ", 2)])]
    #[case::comment_is_no_body("{% comment %}{% a %}", &[(" comment ", 2), (" a ", 15)])]
    #[case::raw_after_python_whitespace(
        "{%\x1Craw %}{{ a }}{% endraw %}",
        &[("\x1Craw ", 2), (" endraw ", 18)]
    )]
    #[case::raw_begin_takes_no_closing_plus("{% raw +%}{% a %}", &[(" raw +", 2), (" a ", 12)])]
    #[case::variable_skips_strings_and_dicts(
        r#"{{ {"a": {"b": "}}"}} }}{{ c }}"#,
        &[(r#" {"a": {"b": "}}"}} "#, 2), (" c ", 26)]
    )]
    fn jinja_tokens(#[case] text: &str, #[case] expected: &[(&str, usize)]) {
        assert_eq!(lexed(text, Language::Jinja), expected);
    }

    #[rstest]
    #[case::jinja_markers_stripped("- block foo-", Language::Jinja, Some("foo"))]
    #[case::django_has_no_markers("- block foo-", Language::Django, Some("block"))]
    #[case::quoted_bit_kept_whole(
        r#" include "my file.html" "#,
        Language::Django,
        Some(r#""my file.html""#)
    )]
    fn tag_argument(
        #[case] content: &str,
        #[case] language: Language,
        #[case] expected: Option<&str>,
    ) {
        let tag = JinjaTag { content, start: 0 };
        assert_eq!(super::tag_argument(&tag, language), expected);
    }
}

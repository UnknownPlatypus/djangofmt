//! The tokens Django's or Jinja's lexer reads before any HTML,
//! including the tags the HTML parser keeps as text, such as in attribute values or `<script>` bodies.

use std::sync::LazyLock;

use markup_fmt::Language;
use markup_fmt::ast::JinjaTag;
use markup_fmt::parser::parse_jinja_tag_name;
use memchr::{memchr, memchr2, memchr2_iter};
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
    static ENDRAW: LazyLock<Regex> = LazyLock::new(|| {
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
        let found = if !django && close == b'%' {
            jinja_tag_closer(&bytes[body..])
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
        match close {
            b'}' if verbatim.is_none() && !in_comment => tokens.push(Token::Var(content)),
            b'%' => {
                let tag = JinjaTag {
                    content,
                    start: body,
                };
                let name = tag_name(&tag, language);
                if django
                    && (verbatim.is_some() || in_comment || matches!(name, "verbatim" | "comment"))
                {
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
                    in_comment = !in_comment && name == "comment";
                }
                tokens.push(Token::Tag { tag, name });
                // Jinja reads a raw body as text up to the `{% endraw %}` it searches for.
                if !django
                    && name == "raw"
                    && content.trim_matches(['-', '+']).trim_matches(is_space) == "raw"
                {
                    let Some(endraw) = ENDRAW.find_at(source, pos) else {
                        break;
                    };
                    let start = endraw.start() + 2;
                    let content = &source[start..endraw.end() - 2];
                    let tag = JinjaTag { content, start };
                    tokens.push(Token::Tag {
                        tag,
                        name: "endraw",
                    });
                    pos = endraw.end();
                }
            }
            _ => {}
        }
    }
    tokens
}

/// The offset of the first `close` byte followed by `}`, or where the search gave up:
/// Django's tokens end at a newline.
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

/// The offset of the first `%}` outside a string literal, which Jinja lexes in a tag.
fn jinja_tag_closer(rest: &[u8]) -> Result<usize, usize> {
    // Few tags quote anything, and with no quote before it the first `%}` closes the tag.
    if let Ok(first) = closer(rest, b'%', false)
        && memchr2(b'"', b'\'', &rest[..first]).is_none()
    {
        return Ok(first);
    }
    let mut quote = None;
    let mut i = 0;
    while i < rest.len() {
        match (quote, rest[i]) {
            (Some(_), b'\\') => i += 1,
            (Some(open), b) if b == open => quote = None,
            (None, b @ (b'"' | b'\'')) => quote = Some(b),
            (None, b'%') if rest.get(i + 1) == Some(&b'}') => return Ok(i),
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
        parse_jinja_tag_name(tag, language)
    }
}

/// The bit after the tag name, such as a block's name, past Jinja's whitespace-control markers.
#[must_use]
pub fn tag_argument<'s>(tag: &JinjaTag<'s>, language: Language) -> Option<&'s str> {
    let content = tag.content.trim_start_matches(['+', '-']);
    // Django has no closing marker, so a trailing `-` is part of its argument.
    let content = if language == Language::Django {
        content
    } else {
        content.trim_end_matches(['+', '-'])
    };
    content.split(is_space).filter(|bit| !bit.is_empty()).nth(1)
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
    fn jinja_tokens(#[case] text: &str, #[case] expected: &[(&str, usize)]) {
        assert_eq!(lexed(text, Language::Jinja), expected);
    }

    #[rstest]
    #[case::jinja_closing_marker("block foo-", Language::Jinja, Some("foo"))]
    #[case::django_trailing_dash(" block foo- ", Language::Django, Some("foo-"))]
    fn tag_argument(
        #[case] content: &str,
        #[case] language: Language,
        #[case] expected: Option<&str>,
    ) {
        let tag = JinjaTag { content, start: 0 };
        assert_eq!(super::tag_argument(&tag, language), expected);
    }
}

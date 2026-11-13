//! The tokens Django's lexer reads before any HTML,
//! including the tags the HTML parser keeps as text, such as in attribute values or `<script>` bodies.

use std::iter;
use std::ops::Deref;
use std::sync::LazyLock;

use markup_fmt::Language;
use markup_fmt::ast::JinjaTag;
use markup_fmt::parser::parse_jinja_tag_name;
use memchr::{memchr_iter, memchr2, memchr2_iter};
use regex::Regex;

use crate::dtl::{self, is_space};

/// A token Django compiles; the lexer drops `{# #}` comments.
#[derive(Debug)]
pub enum Token<'s> {
    /// A `{% %}` with its name: Django's first bit, Jinja's past the whitespace-control markers.
    Tag { tag: JinjaTag<'s>, name: &'s str },
    /// A `{{ }}` expression.
    Var(&'s str),
}

impl<'s> Token<'s> {
    fn tag(content: &'s str, start: usize, language: Language) -> Self {
        let tag = JinjaTag { content, start };
        Self::Tag {
            name: tag_name(&tag, language),
            tag,
        }
    }
}

/// The tokens of a template in source order,
/// less the bodies of `{% verbatim %}`, `{% comment %}` and Jinja's `{% raw %}`.
#[derive(Debug)]
pub struct Tokens<'s>(Vec<Token<'s>>);

/// Jinja's `raw_end` pattern: a `{% endraw %}` with optional whitespace-control markers.
static ENDRAW: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{%[-+]?[\s\x1C-\x1F]*endraw[\s\x1C-\x1F]*[-+]?%\}")
        .expect("endraw pattern is valid")
});

impl<'s> Tokens<'s> {
    /// Lex `source` like Django's `Lexer`, or like Jinja's for any other language,
    /// where a token may span lines and a string literal in a tag may hold a `%}`.
    #[must_use]
    pub fn lex(source: &'s str, language: Language) -> Self {
        // Most templates hold fewer than 512 tokens,
        // and the allocator still serves an allocation of that size from its thread cache.
        let mut tokens = Vec::with_capacity(512);
        if matches!(language, Language::Django) {
            let mut skipped = SkippedBody::default();
            tokens.extend(lex(source, language, 0).filter(|token| skipped.compiles(token)));
            return Self(tokens);
        }
        // Jinja's lexer reads a `{% raw %}` body as text up to the `{% endraw %}` it searches for,
        // so a stray `{%` or `{{` in the body never swallows the closer.
        let mut from = 0;
        loop {
            let mut raw = None;
            for token in lex(source, language, from) {
                if let Token::Tag { tag, name: "raw" } = &token
                    && tag.content.trim_matches(['-', '+']).trim_matches(is_space) == "raw"
                {
                    raw = Some(tag.start);
                }
                tokens.push(token);
                if raw.is_some() {
                    break;
                }
            }
            // The search may start inside the `{% raw %}`, whose content holds no `{`.
            let Some(closer) = raw.and_then(|start| ENDRAW.find_at(source, start)) else {
                return Self(tokens);
            };
            let body = closer.start() + 2;
            tokens.push(Token::tag(&source[body..closer.end() - 2], body, language));
            from = closer.end();
        }
    }
}

impl<'s> Deref for Tokens<'s> {
    type Target = [Token<'s>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Django names a tag by its first bit; Jinja strips whitespace-control markers first.
#[must_use]
pub fn tag_name<'s>(tag: &JinjaTag<'s>, language: Language) -> &'s str {
    if matches!(language, Language::Django) {
        dtl::tag_name(tag.content)
    } else {
        jinja_tag_name(tag, language)
    }
}

/// The bit after the tag name, such as a block's name, past Jinja's whitespace-control markers.
#[must_use]
pub fn tag_argument<'s>(tag: &JinjaTag<'s>, language: Language) -> Option<&'s str> {
    let content = tag.content.trim_start_matches(['+', '-']);
    // Django has no closing marker, so a trailing `-` is part of its argument.
    let content = if matches!(language, Language::Django) {
        content
    } else {
        content.trim_end_matches(['+', '-'])
    };
    content.split(is_space).filter(|bit| !bit.is_empty()).nth(1)
}

/// `parse_jinja_tag_name` on bytes, deferring to the fork for non-ASCII whitespace.
/// The name is the run of `[A-Za-z0-9_]` past the whitespace-control markers and the whitespace.
fn jinja_tag_name<'s>(tag: &JinjaTag<'s>, language: Language) -> &'s str {
    let rest = tag
        .content
        .trim_start_matches(['+', '-'])
        .trim_ascii_start();
    match rest.bytes().next() {
        // `\x0B` and the non-ASCII code points are the whitespace ASCII trimming leaves behind.
        Some(b) if b == b'\x0B' || b >= 0x80 => parse_jinja_tag_name(tag, language),
        _ => {
            let end = rest
                .bytes()
                .position(|b| !(b.is_ascii_alphanumeric() || b == b'_'))
                .unwrap_or(rest.len());
            &rest[..end]
        }
    }
}

/// Where a run of Django's tokens stands in the `{% verbatim %}` and `{% comment %}` bodies.
#[derive(Default)]
struct SkippedBody<'s> {
    /// The lexer's state: the content of the open `{% verbatim %}`, closed by `end` + itself.
    verbatim: Option<&'s str>,
    /// The parser's state: inside a `{% comment %}` body, which `skip_past` drops.
    in_comment: bool,
}

impl<'s> SkippedBody<'s> {
    const fn in_body(&self) -> bool {
        self.verbatim.is_some() || self.in_comment
    }

    /// Whether Django compiles `token`, given the tags before it.
    fn compiles(&mut self, token: &Token<'s>) -> bool {
        let Token::Tag { tag, name } = token else {
            return !self.in_body();
        };
        // Only `verbatim` or `comment` opens a body, so most names pass on their first byte.
        (!self.in_body() && !matches!(name.as_bytes().first(), Some(b'v' | b'c')))
            || self.compiles_slow(tag.content, name)
    }

    #[cold]
    fn compiles_slow(&mut self, content: &'s str, name: &str) -> bool {
        // `token_string[2:-2].strip()`, which the lexer and `skip_past` compare whole.
        let content = content.trim_matches(is_space);
        // The lexer runs before the parser: a verbatim body hides an `{% endcomment %}` within it.
        if let Some(opener) = self.verbatim {
            if content.strip_prefix("end") != Some(opener) {
                return false;
            }
            self.verbatim = None;
        } else if content == "verbatim" || content.starts_with("verbatim ") {
            self.verbatim = Some(content);
        }
        if self.in_comment {
            self.in_comment = content != "endcomment";
            return !self.in_comment;
        }
        self.in_comment = name == "comment";
        true
    }
}

/// The token kinds, indexing the closers `lex` tracks per line.
const TAG: usize = 0;
const VARIABLE: usize = 1;
const COMMENT: usize = 2;

/// Every `{% %}` and `{{ }}` token of Django's `tag_re` in `text` from `from` on,
/// verbatim and comment bodies included.
fn lex(text: &str, language: Language, from: usize) -> impl Iterator<Item = Token<'_>> {
    let jinja = !matches!(language, Language::Django);
    let bytes = text.as_bytes();
    let mut pos = from;
    // The newline the last failed closer search ran into: Django's tokens never span one.
    let mut line_end = usize::MAX;
    // Per closer, whether it is missing from the rest of the line,
    // since never searching twice for it keeps the scan linear.
    let mut unclosed = [false; 3];
    iter::from_fn(move || {
        loop {
            let open = pos + find_tag_open(&bytes[pos..])?;
            if open >= line_end {
                line_end = usize::MAX;
                unclosed = [false; 3];
            }
            // Like the regex, a failed match retries from the next byte,
            // so `{{%` holds a tag when `}}` never comes.
            pos = open + 1;
            let (close, kind) = match bytes[open + 1] {
                b'%' => (b'%', TAG),
                b'{' => (b'}', VARIABLE),
                _ => (b'#', COMMENT),
            };
            if unclosed[kind] {
                continue;
            }
            let body = open + 2;
            let closer = if !jinja {
                find_closer_on_line(&bytes[body..], close).map_err(|end| body + end)
            } else if kind == TAG {
                find_jinja_tag_closer(&bytes[body..]).ok_or(usize::MAX)
            } else {
                find_closer(&bytes[body..], close).ok_or(usize::MAX)
            };
            let len = match closer {
                Ok(len) => len,
                Err(end) => {
                    line_end = end;
                    unclosed[kind] = true;
                    continue;
                }
            };
            pos = body + len + 2;
            let content = &text[body..body + len];
            return Some(match kind {
                TAG => Token::tag(content, body, language),
                VARIABLE => Token::Var(content),
                _ => continue,
            });
        }
    })
}

/// The offset of the first `{%`, `{{` or `{#` in `bytes`.
fn find_tag_open(bytes: &[u8]) -> Option<usize> {
    memchr_iter(b'{', bytes).find(|&i| matches!(bytes.get(i + 1), Some(b'%' | b'{' | b'#')))
}

/// The offset in `line` of the first `close` byte followed by `}`.
fn find_closer(line: &[u8], close: u8) -> Option<usize> {
    memchr_iter(b'}', line.get(1..)?).find(|&i| line[i] == close)
}

/// The offset in `rest` of the first `close` byte followed by `}`,
/// or of the end of the line when no closer comes before it.
fn find_closer_on_line(rest: &[u8], close: u8) -> Result<usize, usize> {
    for i in memchr2_iter(b'}', b'\n', rest) {
        if rest[i] == b'\n' {
            return Err(i);
        }
        if let Some(len) = i.checked_sub(1).filter(|&len| rest[len] == close) {
            return Ok(len);
        }
    }
    Err(rest.len())
}

/// The offset in `body` of the first `%}` outside a string literal, which Jinja lexes in a tag.
fn find_jinja_tag_closer(body: &[u8]) -> Option<usize> {
    // Few tags quote anything, and with no quote before it the first `%}` is the closer.
    let first = find_closer(body, b'%')?;
    if memchr2(b'"', b'\'', &body[..first]).is_none() {
        return Some(first);
    }
    let mut quote = None;
    let mut i = 0;
    while i < body.len() {
        match (quote, body[i]) {
            (Some(_), b'\\') => i += 1,
            (Some(open), b) if b == open => quote = None,
            (None, b @ (b'"' | b'\'')) => quote = Some(b),
            (None, b'%') if body.get(i + 1) == Some(&b'}') => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn lexed(text: &str, language: Language) -> Vec<(&str, usize)> {
        Tokens::lex(text, language)
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

    #[test]
    fn jinja_tag_name_matches_the_parser() {
        for content in ["- if x -", "+for", "set-x", "\u{a0}set", "\x0Bset", "é"] {
            let tag = JinjaTag { content, start: 0 };
            assert_eq!(
                jinja_tag_name(&tag, Language::Jinja),
                parse_jinja_tag_name(&tag, Language::Jinja),
                "{content:?}"
            );
        }
    }
}

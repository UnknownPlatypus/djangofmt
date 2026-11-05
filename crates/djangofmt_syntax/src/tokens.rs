//! Django's view of a template: the tokens its lexer reads before any HTML.
//!
//! The HTML parser keeps the `{% %}` and `{{ }}` inside attribute values, HTML comments and
//! `<script>`, `<style>`, `<pre>` or `<textarea>` bodies as text, but Django compiles them like
//! any other. The rules that read tags lint this stream instead of the tree.

use std::iter;
use std::ops::Deref;
use std::sync::LazyLock;

use markup_fmt::Language;
use markup_fmt::ast::{JinjaInterpolation, JinjaTag};
use markup_fmt::parser::parse_jinja_tag_name;
use memchr::{memchr, memchr_iter};
use regex::Regex;

use crate::dtl::{self, is_space};

/// A token Django compiles; the lexer drops `{# #}` comments.
#[derive(Debug)]
pub enum Token<'s> {
    /// A `{% %}` with its name: Django's first bit, Jinja's past the whitespace-control markers.
    Tag { tag: JinjaTag<'s>, name: &'s str },
    /// A `{{ }}`.
    Var(JinjaInterpolation<'s>),
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

/// The tokens of a template in source order, less the bodies of `{% verbatim %}` (text to
/// Django's lexer), `{% comment %}` (skipped by its parser) and Jinja's `{% raw %}`.
#[derive(Debug)]
pub struct Tokens<'s>(Vec<Token<'s>>);

/// Jinja's `raw_end` pattern: a `{% endraw %}` with optional whitespace-control markers.
static ENDRAW: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{%[-+]?\s*endraw\s*[-+]?%\}").expect("endraw pattern is valid"));

impl<'s> Tokens<'s> {
    /// Lex `source` like Django's `Lexer`, or like Jinja's for any other language, where a token
    /// may span lines.
    #[must_use]
    pub fn lex(source: &'s str, language: Language) -> Self {
        if matches!(language, Language::Django) {
            let mut skipped = SkippedBody::default();
            return Self(
                lex(source, language, 0)
                    .filter(|token| skipped.compiles(token))
                    .collect(),
            );
        }
        // Jinja's lexer reads a `{% raw %}` body as text up to the `{% endraw %}` it searches
        // for, so a stray `{%` or `{{` in the body never swallows the closer.
        let mut tokens = Vec::new();
        let mut from = 0;
        loop {
            let mut raw_end = None;
            for token in lex(source, language, from) {
                if let Token::Tag { tag, name: "raw" } = &token
                    && tag.content.trim_matches(['-', '+']).trim_matches(is_space) == "raw"
                {
                    raw_end = Some(tag.start + tag.content.len() + 2);
                }
                tokens.push(token);
                if raw_end.is_some() {
                    break;
                }
            }
            let Some(closer) = raw_end.and_then(|end| ENDRAW.find_at(source, end)) else {
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

/// `parse_jinja_tag_name` on bytes: the name is the run of `[A-Za-z0-9_]` past the
/// whitespace-control markers and the whitespace. The fork decodes non-ASCII whitespace.
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
            || self.compiles_slow(tag.content)
    }

    #[cold]
    fn compiles_slow(&mut self, content: &'s str) -> bool {
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
        self.in_comment = dtl::tag_name(content) == "comment";
        true
    }
}

/// The token kinds, indexing the closers `lex` tracks per line.
const TAG: usize = 0;
const VARIABLE: usize = 1;
const COMMENT: usize = 2;

/// Every `{% %}` and `{{ }}` token of Django's `tag_re` in `text` from `from` on, verbatim and
/// comment bodies included.
fn lex(text: &str, language: Language, from: usize) -> impl Iterator<Item = Token<'_>> {
    let multiline = !matches!(language, Language::Django);
    let bytes = text.as_bytes();
    // Django's tokens never span a newline, so only the lines holding a `{` opener are tokenized.
    let mut pos = from;
    let mut line_end = from;
    // Per closer, whether it is missing from the rest of the line: never searching twice for it
    // keeps the scan linear.
    let mut unclosed = [false; 3];
    iter::from_fn(move || {
        loop {
            let open = pos + find_tag_open(&bytes[pos..])?;
            if open >= line_end {
                line_end = if multiline {
                    bytes.len()
                } else {
                    memchr(b'\n', &bytes[open..]).map_or(bytes.len(), |nl| open + nl)
                };
                unclosed = [false; 3];
            }
            // Like the regex, a failed match retries from the next byte: `{{%` holds a tag when
            // `}}` never comes.
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
            let Some(len) = find_closer(&bytes[body..line_end], close) else {
                unclosed[kind] = true;
                continue;
            };
            pos = body + len + 2;
            let content = &text[body..body + len];
            return Some(match kind {
                TAG => Token::tag(content, body, language),
                VARIABLE => Token::Var(JinjaInterpolation {
                    expr: content,
                    start: body,
                    trim_prev: false,
                    trim_next: false,
                }),
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn lexed(text: &str, language: Language) -> Vec<(&str, usize)> {
        Tokens::lex(text, language)
            .iter()
            .map(|token| match token {
                Token::Tag { tag, .. } => (tag.content, tag.start),
                Token::Var(var) => (var.expr, var.start),
            })
            .collect()
    }

    #[rstest]
    #[case::two_on_one_line("{% if a %}x{%endif%}", &[(" if a ", 2), ("endif", 13)])]
    #[case::first_closer_wins(r#"{% x "{%}" %}"#, &[(r#" x "{"#, 2)])]
    #[case::variable_kept_comment_dropped("{{ '{% a %}' }} {# {% b %} #}", &[(" '{% a %}' ", 2)])]
    #[case::newline_ends_token("{% a\n{% b %}", &[(" b ", 7)])]
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

    /// The stray `{%` in the raw body would otherwise swallow the `{% endraw %}`.
    #[test]
    fn jinja_tags_span_lines_and_raw_bodies_are_text() {
        let text = "{% if\na %}{%- raw -%}{% b %}{{ c }}{%{% endraw %}{% d %}";
        let names: Vec<_> = Tokens::lex(text, Language::Jinja)
            .iter()
            .map(|token| match token {
                Token::Tag { name, .. } => *name,
                Token::Var(var) => var.expr,
            })
            .collect();
        assert_eq!(names, ["if", "raw", "endraw", "d"]);
    }
}

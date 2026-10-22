//! Django's view of a template: the tokens its lexer reads before any HTML.
//!
//! The HTML parser keeps the `{% %}` and `{{ }}` inside attribute values, HTML comments and
//! `<script>`, `<style>`, `<pre>` or `<textarea>` bodies as text, but Django compiles them like
//! any other. The rules that read tags lint this stream instead of the tree.

use std::iter;
use std::ops::Deref;

use markup_fmt::Language;
use markup_fmt::ast::{JinjaInterpolation, JinjaTag};
use markup_fmt::parser::parse_jinja_tag_name;
use memchr::{memchr, memchr_iter, memrchr};

use crate::dtl::{self, is_space};

/// A token Django compiles; the lexer drops `{# #}` comments.
#[derive(Debug)]
pub enum Token<'s> {
    /// A `{% %}` with its name: Django's first bit, Jinja's past the whitespace-control markers.
    Tag { tag: JinjaTag<'s>, name: &'s str },
    /// A `{{ }}`.
    Var(JinjaInterpolation<'s>),
}

/// The tokens of a template in source order, less the bodies of `{% verbatim %}` (text to
/// Django's lexer), `{% comment %}` (skipped by its parser) and Jinja's `{% raw %}`.
#[derive(Debug)]
pub struct Tokens<'s>(Vec<Token<'s>>);

impl<'s> Tokens<'s> {
    /// Lex `source` like Django's `Lexer`, or like Jinja's for any other language, where a token
    /// may span lines.
    #[must_use]
    pub fn lex(source: &'s str, language: Language) -> Self {
        let multiline = !matches!(language, Language::Django);
        let mut skipped = SkippedBody::default();
        Self(
            lex(source, multiline)
                .filter_map(|raw| match raw {
                    Raw::Tag(tag) => skipped.compiles(tag.content).then(|| Token::Tag {
                        name: tag_name(&tag, language),
                        tag,
                    }),
                    Raw::Var(var) => (!skipped.in_body()).then_some(Token::Var(var)),
                })
                .collect(),
        )
    }

    /// The closer of the block the tag at `opener` opens: the next `end<name>` at its depth.
    #[must_use]
    pub fn closer(&self, opener: usize) -> Option<&JinjaTag<'s>> {
        let Token::Tag { name, .. } = &self.0[opener] else {
            return None;
        };
        let mut depth = 0usize;
        for token in &self.0[opener + 1..] {
            let Token::Tag { tag, name: other } = token else {
                continue;
            };
            if other == name {
                depth += 1;
            } else if other.strip_prefix("end") == Some(*name) {
                if depth == 0 {
                    return Some(tag);
                }
                depth -= 1;
            }
        }
        None
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

/// Where a run of tokens stands in the `{% verbatim %}`, `{% raw %}` and `{% comment %}` bodies.
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

    /// Whether Django compiles the tag holding `content`, given the tags before it.
    fn compiles(&mut self, content: &'s str) -> bool {
        // Outside a body only `verbatim`, `raw` or `comment` opens one, so most tags pass on
        // their first byte; a whitespace-control marker may precede it.
        (!self.in_body()
            && !content
                .trim_start_matches(is_space)
                .starts_with(['v', 'r', 'c', '-', '+']))
            || self.compiles_slow(content)
    }

    #[cold]
    fn compiles_slow(&mut self, content: &'s str) -> bool {
        let content = content.trim_matches(['-', '+']).trim_matches(is_space);
        // The lexer runs before the parser: a verbatim body hides an `{% endcomment %}` within it.
        if let Some(opener) = self.verbatim {
            if content.strip_prefix("end") != Some(opener) {
                return false;
            }
            self.verbatim = None;
        } else if content == "verbatim" || content.starts_with("verbatim ") || content == "raw" {
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

/// A token before the verbatim and comment handling.
enum Raw<'s> {
    Tag(JinjaTag<'s>),
    Var(JinjaInterpolation<'s>),
}

/// The three closers `lex` tracks per line.
const TAG: usize = 0;
const VARIABLE: usize = 1;
const COMMENT: usize = 2;

/// Every `{% %}` and `{{ }}` token of Django's `tag_re` in `text`.
fn lex(text: &str, multiline: bool) -> impl Iterator<Item = Raw<'_>> {
    let bytes = text.as_bytes();
    // Django's tokens never span a newline, so only the lines holding a `{%` are tokenized.
    let mut pos = 0;
    let mut line_end = 0;
    // Per closer, whether it is missing from the rest of the line: never searching twice for it
    // keeps the scan linear.
    let mut unclosed = [false; 3];
    iter::from_fn(move || {
        loop {
            if pos == line_end {
                let next_tag = pos + find_tag_open(&bytes[pos..])?;
                if multiline {
                    line_end = bytes.len();
                } else {
                    // The line starts on the newline before it, which opens no token.
                    pos = memrchr(b'\n', &bytes[pos..next_tag]).map_or(pos, |nl| pos + nl);
                    line_end =
                        memchr(b'\n', &bytes[next_tag..]).map_or(bytes.len(), |nl| next_tag + nl);
                    unclosed = [false; 3];
                }
            }
            let Some(brace) = memchr(b'{', &bytes[pos..line_end]) else {
                pos = line_end;
                continue;
            };
            let open = pos + brace;
            // Like the regex, a failed match retries from the next byte: `{{%` holds a tag when
            // `}}` never comes.
            pos = open + 1;
            let (close, kind) = match bytes.get(open + 1) {
                Some(b'%') => (b'%', TAG),
                Some(b'{') => (b'}', VARIABLE),
                Some(b'#') => (b'#', COMMENT),
                _ => continue,
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
            match kind {
                TAG => {
                    return Some(Raw::Tag(JinjaTag {
                        content,
                        start: body,
                    }));
                }
                VARIABLE => {
                    return Some(Raw::Var(JinjaInterpolation {
                        expr: content,
                        start: body,
                        trim_prev: false,
                        trim_next: false,
                    }));
                }
                _ => {}
            }
        }
    })
}

/// The offset of the first `{%` or `{{` in `bytes`.
fn find_tag_open(bytes: &[u8]) -> Option<usize> {
    memchr_iter(b'{', bytes).find(|&i| matches!(bytes.get(i + 1), Some(b'%' | b'{')))
}

/// The offset in `line` of the first `close` byte followed by `}`.
fn find_closer(line: &[u8], close: u8) -> Option<usize> {
    memchr_iter(b'}', line.get(1..)?).find(|&i| line[i] == close)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn tags(text: &str, language: Language) -> Vec<(&str, usize)> {
        Tokens::lex(text, language)
            .iter()
            .filter_map(|token| match token {
                Token::Tag { tag, .. } => Some((tag.content, tag.start)),
                Token::Var(_) => None,
            })
            .collect()
    }

    #[rstest]
    #[case::two_on_one_line("{% if a %}x{%endif%}", &[(" if a ", 2), ("endif", 13)])]
    #[case::first_closer_wins(r#"{% x "{%}" %}"#, &[(r#" x "{"#, 2)])]
    #[case::variable_and_comment_skipped("{{ '{% a %}' }} {# {% b %} #}", &[])]
    #[case::multi_line_not_a_tag("{% if\na %}", &[])]
    #[case::unclosed_variable_retries("{{% a %}", &[(" a ", 3)])]
    #[case::variable_ends_at_newline("{{ a\n{% b %} }}", &[(" b ", 7)])]
    #[case::verbatim_body_dropped(
        "{% verbatim x %}{% a %}{% endverbatim %}{% endverbatim x %}{% b %}",
        &[(" verbatim x ", 2), (" endverbatim x ", 42), (" b ", 61)]
    )]
    #[case::comment_body_dropped(
        r#"{% comment "note" %}{% a %}{% endcomment %}{% b %}"#,
        &[(r#" comment "note" "#, 2), (" endcomment ", 29), (" b ", 45)]
    )]
    fn django_tags(#[case] text: &str, #[case] expected: &[(&str, usize)]) {
        assert_eq!(tags(text, Language::Django), expected);
    }

    #[test]
    fn jinja_tag_name_matches_the_parser() {
        for content in [
            "- if x -",
            "+for",
            " raw ",
            "",
            "set-x",
            "\u{a0}set",
            "\x0Bset",
            "é",
        ] {
            let tag = JinjaTag { content, start: 0 };
            assert_eq!(
                jinja_tag_name(&tag, Language::Jinja),
                parse_jinja_tag_name(&tag, Language::Jinja),
                "{content:?}"
            );
        }
    }

    #[test]
    fn jinja_tags_span_lines_and_skip_raw() {
        let text = "{% if\na %}{%- raw -%}{% b %}{{ c }}{% endraw %}";
        let names: Vec<_> = Tokens::lex(text, Language::Jinja)
            .iter()
            .map(|token| match token {
                Token::Tag { name, .. } => *name,
                Token::Var(var) => var.expr,
            })
            .collect();
        assert_eq!(names, ["if", "raw", "endraw"]);
    }

    #[test]
    fn variables_are_lexed_outside_skipped_bodies() {
        let tokens = Tokens::lex(
            "{{ a }}{% comment %}{{ b }}{% endcomment %}",
            Language::Django,
        );
        let vars: Vec<_> = tokens
            .iter()
            .filter_map(|token| match token {
                Token::Var(var) => Some(var.expr),
                Token::Tag { .. } => None,
            })
            .collect();
        assert_eq!(vars, [" a "]);
    }

    #[test]
    fn closer_skips_nested_blocks_and_intermediate_tags() {
        let tokens = Tokens::lex(
            "{% if a %}{% if b %}{% else %}{% endif %}{% elif c %}{% endif %}{% endif %}",
            Language::Django,
        );
        assert_eq!(tokens.closer(0).map(|tag| tag.start), Some(55));
        assert_eq!(tokens.closer(1).map(|tag| tag.start), Some(32));
        assert!(tokens.closer(2).is_none());
    }
}

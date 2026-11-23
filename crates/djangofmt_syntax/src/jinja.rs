//! Jinja's token stream: multi-line tokens, strings and braces inside a token, and `{% raw %}`.

use std::sync::LazyLock;

use markup_fmt::Language;
use markup_fmt::ast::JinjaTag;
use memchr::{memchr, memchr_iter, memchr3};
use regex::Regex;

use crate::tokens::{Token, tag_name};

/// Lex `source` like Jinja's lexer, dropping comments and the body of `{% raw %}`.
pub fn lex(source: &str) -> Vec<Token<'_>> {
    // Jinja's `raw_begin` and `raw_end`, where Python's `\s` also matches `\x1C` to `\x1F`.
    static RAW_BEGIN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\A\{%[-+]?[\s\x1C-\x1F]*raw[\s\x1C-\x1F]*-?%\}\z").expect("valid pattern")
    });
    static RAW_END: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\{%[-+]?[\s\x1C-\x1F]*endraw[\s\x1C-\x1F]*[-+]?%\}").expect("valid pattern")
    });
    let bytes = source.as_bytes();
    let (mut tokens, mut pos) = (Vec::new(), 0);
    // Per token kind, the offset before which its closer is known missing, keeping the scan linear.
    let mut missing = [0; 3];
    while let Some(i) = memchr(b'{', &bytes[pos..]) {
        let open = pos + i;
        // A failed match retries from the next byte.
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
        let found = if close == b'#' {
            closer(&bytes[body..], close)
        } else {
            jinja_closer(&bytes[body..], close)
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
            if close == b'}' {
                tokens.push(Token::Var(content));
            }
            continue;
        }
        let tag = JinjaTag {
            content,
            start: body,
        };
        let name = tag_name(&tag, Language::Jinja);
        tokens.push(Token::Tag { tag, name });
        // Jinja reads a raw body as text up to the `{% endraw %}` it searches for, lexed next.
        if name == "raw" && RAW_BEGIN.is_match(&source[open..pos]) {
            pos = RAW_END
                .find_at(source, pos)
                .map_or(source.len(), |end| end.start());
        }
    }
    tokens
}

/// The offset of the first `close` byte followed by `}`, or where the search gave up.
fn closer(rest: &[u8], close: u8) -> Result<usize, usize> {
    for i in memchr_iter(b'}', rest) {
        if i > 0 && rest[i - 1] == close {
            return Ok(i - 1);
        }
    }
    Err(rest.len())
}

/// The offset of the first `close` byte followed by `}` outside string literals and `{ }` pairs,
/// which Jinja lexes in a `{% %}` or `{{ }}`.
fn jinja_closer(rest: &[u8], close: u8) -> Result<usize, usize> {
    let first = closer(rest, close)?;
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

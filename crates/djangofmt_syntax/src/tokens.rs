//! The tokens Django's or Jinja's lexer reads before any HTML,
//! including the tags the HTML parser keeps as text, such as in attribute values or `<script>` bodies.

use dtl_lexer::START_TAG_LEN;
use dtl_lexer::core::{Lexer, TokenType};
use dtl_lexer::types::TemplateString;
use markup_fmt::Language;
use markup_fmt::ast::JinjaTag;

use crate::dtl::{self, is_space};
use crate::jinja;

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
    if language != Language::Django {
        return jinja::lex(source);
    }
    let template = TemplateString(source);
    let mut tokens = Vec::new();
    // Django's lexer skips a verbatim body, but its parser skips a comment body.
    let mut in_comment = false;
    for token in Lexer::new(template) {
        match token.token_type {
            TokenType::Variable if !in_comment => tokens.push(Token::Var(token.content(template))),
            TokenType::Tag => {
                let content = token.content(template);
                if in_comment && content.trim_matches(is_space) != "endcomment" {
                    continue;
                }
                let name = dtl::tag_name(content);
                in_comment = name == "comment";
                let start = token.at.0 + START_TAG_LEN;
                tokens.push(Token::Tag {
                    tag: JinjaTag { content, start },
                    name,
                });
            }
            _ => {}
        }
    }
    tokens
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
    #[case::verbatim_after_python_whitespace(
        "{%\x1Cverbatim %}{{ a }}{% endverbatim %}",
        &[("\x1Cverbatim ", 2), (" endverbatim ", 23)]
    )]
    #[case::verbatim_end_hidden_in_comment(
        "{% verbatim %}{# {% endverbatim %} #}{{ a }}{% endverbatim %}{% b %}",
        &[(" verbatim ", 2), (" endverbatim ", 46), (" b ", 63)]
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

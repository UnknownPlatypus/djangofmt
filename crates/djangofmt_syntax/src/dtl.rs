//! Django template language lexing, ported from Django's own regexes.
//!
//! Python's `\s` and `\w` differ from the `regex` crate's, so both patterns spell them out:
//! `\s` is [`is_space`], `\w` is `[\p{L}\p{N}_]`.

use std::fmt;
use std::sync::LazyLock;

use regex::Regex;

/// Python's `\s` and `str.isspace()`: Unicode `White_Space` plus U+001C to U+001F.
#[must_use]
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\x1C'..='\x1F').contains(&c)
}

/// Whether the only whitespace in `body` is single plain spaces.
#[must_use]
pub fn is_single_spaced(body: &str) -> bool {
    let mut after_space = false;
    body.chars().all(|c| {
        let single = if c == ' ' { !after_space } else { !is_space(c) };
        after_space = c == ' ';
        single
    })
}

/// Whether whitespace touches a `|`, the only whitespace [`FilterExpression`] drops when printed.
#[must_use]
pub fn has_spaced_pipe(expr: &str) -> bool {
    expr.match_indices('|')
        .any(|(i, _)| expr[..i].ends_with(is_space) || expr[i + 1..].starts_with(is_space))
}

/// The bits of a `{% %}` tag body, as Django's `smart_split` yields them.
///
/// A quoted string stays whole, glued to the characters touching it, and keeps its quotes.
#[must_use]
pub fn bits(content: &str) -> Vec<&str> {
    // `smart_split_re`, django/utils/text.py
    static SMART_SPLIT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"(?:[^\s\x1C-\x1F'"]*(?:(?:"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')[^\s\x1C-\x1F'"]*)+)|[^\s\x1C-\x1F]+"#,
        )
        .expect("smart_split pattern is valid")
    });
    // Without a quote only the `\S+` branch can match, so a plain split gives the same bits faster.
    if !content.contains(['"', '\'']) {
        return content
            .split(is_space)
            .filter(|bit| !bit.is_empty())
            .collect();
    }
    SMART_SPLIT.find_iter(content).map(|m| m.as_str()).collect()
}

/// A `{{ }}` body: a constant or variable followed by its filters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterExpression<'a> {
    /// The head, a string constant (`"x"`, `_("x")`) or a variable (`user.name`, `1.5`).
    pub var: &'a str,
    pub filters: Vec<Filter<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter<'a> {
    pub name: &'a str,
    pub arg: Option<&'a str>,
}

/// Lex a `{{ }}` body like Django's `FilterExpression`, or [`None`] where Django raises.
#[must_use]
pub fn filter_expression(expr: &str) -> Option<FilterExpression<'_>> {
    // `filter_re` as of Django 6.0, django/template/base.py
    static FILTER: LazyLock<Regex> = LazyLock::new(|| {
        let strdq = r#""[^"\\]*(?:\\.[^"\\]*)*""#;
        let strsq = r"'[^'\\]*(?:\\.[^'\\]*)*'";
        let constant = format!(r"(?:_\({strdq}\)|_\({strsq}\)|{strdq}|{strsq})");
        let word = r"\p{L}\p{N}_";
        let var = format!(r"[{word}.+\-]+");
        let space = r"[\s\x1C-\x1F]";
        Regex::new(&format!(
            r"^(?P<constant>{constant})|^(?P<var>{var})|{space}*\|{space}*(?P<filter_name>[{word}]+)(?::(?:(?P<constant_arg>{constant})|(?P<var_arg>{var})))?"
        ))
        .expect("filter pattern is valid")
    });

    let mut matches = FILTER.captures_iter(expr);
    // The first match must be the head, which only matches at the start.
    let head = matches.next()?;
    let var = head
        .name("constant")
        .or_else(|| head.name("var").filter(|var| is_variable(var.as_str())))?;
    let mut filters = Vec::new();
    let mut upto = var.end();
    for filter in matches {
        let whole = filter.get_match();
        if whole.start() != upto {
            return None;
        }
        let var_arg = filter.name("var_arg");
        if var_arg.is_some_and(|arg| !is_variable(arg.as_str())) {
            return None;
        }
        filters.push(Filter {
            name: filter.name("filter_name")?.as_str(),
            arg: filter
                .name("constant_arg")
                .or(var_arg)
                .map(|arg| arg.as_str()),
        });
        upto = whole.end();
    }
    (upto == expr.len()).then_some(FilterExpression {
        var: var.as_str(),
        filters,
    })
}

/// Whether Django's `Variable` accepts a `[\w.+-]+` match: a number, or a lookup path with no
/// `+` or `-` and no segment starting with `_`.
fn is_variable(var: &str) -> bool {
    !(var.starts_with('_') || var.contains("._") || var.contains(['+', '-'])) || is_number(var)
}

/// Whether `Variable` reads `var` as a number: Python's `float()` when it holds a `.` or an `e`,
/// `int()` otherwise, and never with a trailing `.`.
fn is_number(var: &str) -> bool {
    let bytes = var.as_bytes();
    // Python allows a `_` digit separator only between two digits.
    let separators_ok = var.match_indices('_').all(|(i, _)| {
        i > 0 && bytes[i - 1].is_ascii_digit() && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)
    });
    let digits = var.replace('_', "");
    separators_ok
        && if var.contains(['.', 'e', 'E']) {
            !var.ends_with('.') && digits.parse::<f64>().is_ok()
        } else {
            let unsigned = digits.strip_prefix(['+', '-']).unwrap_or(&digits);
            !unsigned.is_empty() && unsigned.bytes().all(|b| b.is_ascii_digit())
        }
}

/// Prints `var|name:arg|name`, without the whitespace Django allows around `|`.
impl fmt::Display for FilterExpression<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.var)?;
        for filter in &self.filters {
            write!(f, "|{}", filter.name)?;
            if let Some(arg) = filter.arg {
                write!(f, ":{arg}")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::whitespace_runs("if breakfast  ==  'egg  mcmuffin'", &["if", "breakfast", "==", "'egg  mcmuffin'"])]
    #[case::glued_quoted_run(r#"x"a b"y"#, &[r#"x"a b"y"#])]
    #[case::unterminated_quote(r#"a "b c"#, &["a", r#""b"#, "c"])]
    #[case::python_whitespace_unquoted("a\u{a0}b\u{1c}c", &["a", "b", "c"])]
    #[case::python_whitespace_quoted("'a'\u{a0}b\u{1c}c", &["'a'", "b", "c"])]
    #[case::blank(" \t\n", &[])]
    // The `smart_split` docstring examples.
    #[case::escaped_quote_in_double_quotes(r#"This is "a person\'s" test."#, &["This", "is", r#""a person\'s""#, "test."])]
    #[case::escaped_quote_in_single_quotes(r"Another 'person\'s' test.", &["Another", r"'person\'s'", "test."])]
    #[case::escaped_quotes_at_edge(r#"A "\"funky\" style" test."#, &["A", r#""\"funky\" style""#, "test."])]
    fn bits_cases(#[case] content: &str, #[case] expected: &[&str]) {
        assert_eq!(bits(content), expected);
    }

    #[rstest]
    #[case::string_constant("'egg'", Some("'egg'"))]
    #[case::translated_constant(r#"_("x") | upper"#, Some(r#"_("x")|upper"#))]
    #[case::dotted_var("engine..name", Some("engine..name"))]
    #[case::exponent_sign("1e+5", Some("1e+5"))]
    #[case::signed_number("-1.5", Some("-1.5"))]
    #[case::digit_separator("-1_000", Some("-1_000"))]
    #[case::trailing_dot("-1.", None)]
    #[case::minus_in_name("total-1", None)]
    #[case::minus_in_arg("a|default:b-c", None)]
    #[case::underscore_prefix("_private", None)]
    #[case::underscore_attribute("a._b", None)]
    #[case::filter("egg | crack", Some("egg|crack"))]
    #[case::constant_arg("egg | crack:'fully'", Some("egg|crack:'fully'"))]
    #[case::var_arg("egg | crack:amount", Some("egg|crack:amount"))]
    #[case::python_classes("é\u{a0}|\u{1c}ü", Some("é|ü"))]
    #[case::empty("", None)]
    #[case::invalid_start("?egg | crack", None)]
    #[case::invalid_end("egg | crack?", None)]
    #[case::invalid_middle("engines[0].name | length", None)]
    #[case::filter_as_head("|a|b", None)]
    #[case::space_after_arg_separator("a|b: c", None)]
    #[case::space_without_separator("a .b", None)]
    #[case::mark_is_not_word_char("x\u{0903}|upper", None)]
    fn filter_expression_cases(#[case] expr: &str, #[case] expected: Option<&str>) {
        assert_eq!(
            filter_expression(expr).map(|fe| fe.to_string()).as_deref(),
            expected
        );
    }

    #[test]
    fn filter_expression_keeps_string_arg_whole() {
        assert_eq!(
            filter_expression(r#"a|default:"x | y"|upper"#),
            Some(FilterExpression {
                var: "a",
                filters: vec![
                    Filter {
                        name: "default",
                        arg: Some(r#""x | y""#),
                    },
                    Filter {
                        name: "upper",
                        arg: None,
                    },
                ],
            })
        );
    }

    #[rstest]
    #[case::single_spaces("load i18n static", true)]
    #[case::double_space("load  i18n", false)]
    #[case::other_whitespace("load\u{a0}i18n", false)]
    fn is_single_spaced_cases(#[case] body: &str, #[case] expected: bool) {
        assert_eq!(is_single_spaced(body), expected);
    }

    #[rstest]
    #[case::before_pipe("a |b", true)]
    #[case::after_pipe("a|\u{a0}b", true)]
    #[case::space_away_from_pipes(r#"d|date:"N j, Y""#, false)]
    fn has_spaced_pipe_cases(#[case] expr: &str, #[case] expected: bool) {
        assert_eq!(has_spaced_pipe(expr), expected);
    }
}

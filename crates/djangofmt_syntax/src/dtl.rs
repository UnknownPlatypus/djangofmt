//! Django template language lexing, ported from Django's own regexes.
//!
//! Python's `\s` and `\w` differ from the `regex` crate's, so both patterns spell them out.
//! `\s` is [`is_space`], and `\w` is `[\p{L}\p{N}_]`.

use std::fmt;
use std::sync::LazyLock;

use regex::Regex;

/// Python's `\s` and `str.isspace()`: Unicode `White_Space` plus U+001C to U+001F.
#[must_use]
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\x1C'..='\x1F').contains(&c)
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

/// The expression without the whitespace Django allows around `|`.
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
        let var_arg = filter.name("var_arg");
        if whole.start() != upto || var_arg.is_some_and(|arg| !is_variable(arg.as_str())) {
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

/// A `{{ }}` body without the whitespace Django allows around `|`,
/// or [`None`] where Django's `FilterExpression` raises.
#[must_use]
pub fn compact_filter_expression(expr: &str) -> Option<String> {
    filter_expression(expr).map(|expression| expression.to_string())
}

/// Whether Django's `Variable` accepts a `[\w.+-]+` match.
/// That is a number, or a lookup path with no `+`, no `-` and no segment starting with `_`.
fn is_variable(var: &str) -> bool {
    !(var.starts_with('_') || var.contains("._") || var.contains(['+', '-'])) || is_number(var)
}

/// Whether `Variable` reads `var` as a number: Python's `float()` when it holds a `.` or an `e`,
/// `int()` otherwise, and never with a trailing `.`.
/// Rejecting Python's `_` digit separators only leaves such rare numbers verbatim.
fn is_number(var: &str) -> bool {
    if var.contains(['.', 'e', 'E']) {
        !var.ends_with('.') && var.parse::<f64>().is_ok()
    } else {
        let unsigned = var.strip_prefix(['+', '-']).unwrap_or(var);
        !unsigned.is_empty() && unsigned.bytes().all(|b| b.is_ascii_digit())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::glued_quoted_run(r#"x"a b"y"#, &[r#"x"a b"y"#])]
    #[case::unterminated_quote(r#"a "b c"#, &["a", r#""b"#, "c"])]
    #[case::python_whitespace_unquoted("a\u{a0}b\u{1c}c", &["a", "b", "c"])]
    #[case::python_whitespace_quoted("'a'\u{a0}b\u{1c}c", &["'a'", "b", "c"])]
    #[case::blank(" \t\n", &[])]
    // The `smart_split` docstring examples.
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
    #[case::trailing_dot("-1.", None)]
    #[case::minus_in_name("total-1", None)]
    #[case::minus_in_arg("a|default:b-c", None)]
    #[case::underscore_prefix("_private", None)]
    #[case::underscore_attribute("a._b", None)]
    #[case::filter("egg | crack", Some("egg|crack"))]
    #[case::constant_arg("egg | crack:'fully'", Some("egg|crack:'fully'"))]
    #[case::var_arg("egg | crack:amount", Some("egg|crack:amount"))]
    #[case::pipe_in_string_arg(r#"a | default:"x | y""#, Some(r#"a|default:"x | y""#))]
    #[case::python_classes("é\u{a0}|\u{1c}ü", Some("é|ü"))]
    #[case::empty("", None)]
    #[case::invalid_start("?egg | crack", None)]
    #[case::invalid_end("egg | crack?", None)]
    #[case::invalid_middle("engines[0].name | length", None)]
    #[case::space_after_arg_separator("a|b: c", None)]
    #[case::space_without_separator("a .b", None)]
    #[case::mark_is_not_word_char("x\u{0903}|upper", None)]
    fn compact_filter_expression_cases(#[case] expr: &str, #[case] expected: Option<&str>) {
        assert_eq!(compact_filter_expression(expr).as_deref(), expected);
    }
}

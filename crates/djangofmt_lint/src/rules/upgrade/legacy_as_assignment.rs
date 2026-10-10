use std::borrow::Cow;
use std::ops::Range;

use djangofmt_syntax::dtl::{bits, filter_expression, is_word};
use djangofmt_syntax::span;
use markup_fmt::ast::JinjaTag;

use crate::Checker;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `{% with %}` and `{% blocktranslate %}` tags assigning variables as `value as name`.
///
/// ## Why is this bad?
/// Django 1.3 introduced the `name=value` form, the only one `{% include %}` accepts. The older
/// `value as name` form, chained with `and`, still works in `{% with %}` and
/// `{% blocktranslate %}`, and Django's documentation calls it the previous, more verbose format.
/// Writing `name=value` everywhere keeps one spelling across templates.
///
/// The `with` and `count` options of `{% blocktranslate %}` and `{% blocktrans %}` are checked too.
/// A missing `and` between two `value as name` pairs, which Django rejects, is rewritten as well,
/// since the intent is clear.
///
/// ## Example
/// ```html
/// {% with business.employees.count as total %}
///     {{ total }} employee{{ total|pluralize }}
/// {% endwith %}
/// ```
///
/// Use instead:
/// ```html
/// {% with total=business.employees.count %}
///     {{ total }} employee{{ total|pluralize }}
/// {% endwith %}
/// ```
///
/// ## References
/// - [Django template tags: `with`](https://docs.djangoproject.com/en/stable/ref/templates/builtins/#std-templatetag-with)
/// - [Django documentation: `blocktranslate`](https://docs.djangoproject.com/en/stable/topics/i18n/translation/#std-templatetag-blocktranslate)
/// - [Django 1.3 release notes: built-in template tags](https://docs.djangoproject.com/en/stable/releases/1.3/#improvements-to-built-in-template-tags)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct LegacyAsAssignment<'a> {
    pub tag: &'a str,
}

impl Violation for LegacyAsAssignment<'_> {
    const RULE: Rule = Rule::LegacyAsAssignment;
    const CATEGORY: RuleCategory = RuleCategory::Upgrade;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        format!("Legacy `as` assignment in `{{% {} %}}`", self.tag).into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some("Use `name=value` instead".into())
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Rewrite as `name=value`")
    }
}

/// The options of `{% blocktranslate %}`, which may follow its `with` and `count` clauses.
const BLOCKTRANSLATE_OPTIONS: [&str; 5] = ["with", "count", "context", "trimmed", "asvar"];

/// The caller guarantees a `{% with %}`.
pub fn check_with(checker: &Checker<'_>, tag: &JinjaTag<'_>) {
    if checker.is_django() && has_as_bit(tag.content) {
        check_assignments(checker, tag.content, with_clause);
    }
}

/// The caller guarantees a `{% blocktranslate %}` or `{% blocktrans %}` opener.
pub fn check_blocktranslate(checker: &Checker<'_>, tag: &JinjaTag<'_>) {
    if checker.is_django() && has_as_bit(tag.content) {
        check_assignments(checker, tag.content, blocktranslate_clauses);
    }
}

/// Whether `content` holds an `as` bit, read on bytes since every `with` and `blocktranslate`
/// gets here: a bit is bounded by whitespace, which no ASCII graphic byte is.
fn has_as_bit(content: &str) -> bool {
    let bytes = content.as_bytes();
    let bounds = |i: Option<usize>| {
        !i.and_then(|i| bytes.get(i))
            .is_some_and(u8::is_ascii_graphic)
    };
    content
        .match_indices("as")
        .any(|(i, _)| bounds(i.checked_sub(1)) && bounds(Some(i + 2)))
}

#[cold]
fn check_assignments<'s>(
    checker: &Checker<'_>,
    content: &'s str,
    clauses: fn(&[&'s str]) -> Vec<LegacyClause<'s>>,
) {
    let bits = bits(content);
    // Django lexes such a tag into other bits than these.
    if bits.iter().any(|bit| opens_translation(bit)) {
        return;
    }
    // One edit per clause: `(span, name=value pairs)`.
    let mut rewrites = clauses(&bits).into_iter().map(|clause| {
        let start = checker.source_offset(bits[clause.range.start]);
        let range = span(
            start,
            checker.source_end(bits[clause.range.end - 1]) - start,
        );
        let pairs: Vec<String> = clause
            .pairs
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect();
        (range, pairs.join(" "))
    });
    let Some((range, pairs)) = rewrites.next() else {
        return;
    };
    let edits = rewrites.map(|(range, pairs)| Edit::replacement(pairs, range));
    checker
        .report_diagnostic(&LegacyAsAssignment { tag: bits[0] }, range)
        .set_fix(Fix::safe_edits(Edit::replacement(pairs, range), edits));
}

/// `(name, value)` pairs in source order.
type Pairs<'a> = Vec<(&'a str, &'a str)>;

/// A run of `value as name` pairs and the `and`s between them.
struct LegacyClause<'a> {
    range: Range<usize>,
    pairs: Pairs<'a>,
}

/// The arguments of `{% with %}` when they are all legacy pairs, which Django requires them to be
/// once the first one is.
fn with_clause<'a>(bits: &[&'a str]) -> Vec<LegacyClause<'a>> {
    match assignments(bits, 1, |_| true) {
        Some((pairs, true, end)) if end == bits.len() => vec![LegacyClause {
            range: 1..end,
            pairs,
        }],
        _ => Vec::new(),
    }
}

/// The legacy `with` and `count` clauses of `{% blocktranslate %}`, none when Django rejects it.
fn blocktranslate_clauses<'a>(bits: &[&'a str]) -> Vec<LegacyClause<'a>> {
    let mut legacy = Vec::new();
    let mut seen = 0u8;
    let mut i = 1;
    while let Some(&option) = bits.get(i) {
        let Some(index) = BLOCKTRANSLATE_OPTIONS
            .iter()
            .position(|&known| known == option)
        else {
            return Vec::new();
        };
        if seen & (1 << index) != 0 {
            return Vec::new();
        }
        seen |= 1 << index;
        i += 1;
        match option {
            "with" | "count" => {
                // After a pair without `and`, Django reads an option word as the next option.
                let Some((pairs, is_legacy, end)) =
                    assignments(bits, i, |bit| !BLOCKTRANSLATE_OPTIONS.contains(&bit))
                else {
                    return Vec::new();
                };
                if option == "count" && pairs.len() != 1 {
                    return Vec::new();
                }
                if is_legacy {
                    legacy.push(LegacyClause {
                        range: i..end,
                        pairs,
                    });
                }
                i = end;
            }
            "context" if bits.get(i).is_some_and(|bit| compiles(bit)) => i += 1,
            "asvar" if i < bits.len() => i += 1,
            "trimmed" => {}
            _ => return Vec::new(),
        }
    }
    legacy
}

/// The `(name, value)` pairs Django's `token_kwargs(bits[i:], support_legacy=True)` reads, whether
/// they are legacy, and the index after them; [`None`] when it reads none, a value does not
/// compile, or a legacy name has no `name=value` spelling.
fn assignments<'a>(
    bits: &[&'a str],
    mut i: usize,
    joins: impl Fn(&str) -> bool,
) -> Option<(Pairs<'a>, bool, usize)> {
    let mut pairs = Vec::new();
    // Django's `kwarg_re` decides the format on the first bit.
    while let Some((name, value)) = bits.get(i).and_then(|bit| kwarg_value(bit)) {
        if !compiles(value) {
            return None;
        }
        pairs.push((name, value));
        i += 1;
    }
    if !pairs.is_empty() {
        return Some((pairs, false, i));
    }
    while let [value, "as", name, ..] = bits[i..] {
        if !compiles(value) || !is_word(name) {
            return None;
        }
        pairs.push((name, value));
        i += 3;
        match bits.get(i) {
            Some(&"and") => i += 1,
            // Django stops at a pair without `and`; the typo has one reading when a value follows.
            Some(&next) if next != "as" && joins(next) => {}
            _ => break,
        }
    }
    (!pairs.is_empty()).then_some((pairs, true, i))
}

/// The `(name, value)` of a `name=value` bit, read like Django's `kwarg_re`.
fn kwarg_value(bit: &str) -> Option<(&str, &str)> {
    bit.split_once('=')
        .filter(|(name, value)| is_word(name) && !value.is_empty())
}

/// Whether Django's `compile_filter` lexes `value`.
fn compiles(value: &str) -> bool {
    filter_expression(value).is_some()
}

/// Whether Django's `split_contents` joins `bit` to the bits after it: a `_("` it does not close.
fn opens_translation(bit: &str) -> bool {
    let bytes = bit.as_bytes();
    matches!(bytes, [b'_', b'(', quote @ (b'"' | b'\''), ..] if !bytes.ends_with(&[*quote, b')']))
}

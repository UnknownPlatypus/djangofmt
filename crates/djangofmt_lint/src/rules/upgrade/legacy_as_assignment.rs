use std::borrow::Cow;
use std::ops::Range;

use djangofmt_syntax::dtl::{bits, filter_expression, first_bit_is, is_word};
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

pub fn check(checker: &Checker<'_>, tag: &JinjaTag<'_>) {
    if checker.is_django() && may_name_target(tag.content) {
        check_assignments(checker, tag.content);
    }
}

/// Whether the tag name in `content` may be `with`, `blocktrans` or `blocktranslate`, read from
/// its first bytes since every tag gets here; any unusual spacing is left to [`first_bit_is`].
fn may_name_target(content: &str) -> bool {
    match content.as_bytes() {
        [b' ', b'w', ..] => true,
        [b' ', b'b', rest @ ..] => rest.starts_with(b"lockt"),
        [b' ', byte, ..] => !byte.is_ascii_graphic(),
        _ => true,
    }
}

#[cold]
fn check_assignments(checker: &Checker<'_>, content: &str) {
    let is_with = first_bit_is(content, "with");
    let is_target =
        is_with || first_bit_is(content, "blocktranslate") || first_bit_is(content, "blocktrans");
    // A legacy pair has an `as` bit, so the other tags are not lexed.
    if !is_target || !content.contains("as") {
        return;
    }
    let bits = bits(content);
    // Django lexes such a tag into other bits than these.
    if bits.iter().any(|bit| opens_translation(bit)) {
        return;
    }
    let legacy = if is_with {
        with_clause(&bits)
    } else {
        blocktranslate_clauses(&bits)
    };
    let (Some(first), Some(last)) = (legacy.first(), legacy.last()) else {
        return;
    };

    let mut pieces: Vec<Cow<'_, str>> = Vec::new();
    let mut next = first.bits.start;
    for clause in &legacy {
        pieces.extend(bits[next..clause.bits.start].iter().map(|&bit| bit.into()));
        pieces.extend(
            clause
                .pairs
                .iter()
                .map(|(name, value)| format!("{name}={value}").into()),
        );
        next = clause.bits.end;
    }
    let start = checker.source_offset(bits[first.bits.start]);
    let range = span(start, checker.source_end(bits[last.bits.end - 1]) - start);
    checker
        .report_diagnostic(&LegacyAsAssignment { tag: bits[0] }, range)
        .set_fix(Fix::safe_edit(Edit::replacement(pieces.join(" "), range)));
}

/// A run of `value as name` pairs and the `and`s between them.
struct LegacyClause<'a> {
    bits: Range<usize>,
    /// `(name, value)` in source order.
    pairs: Vec<(&'a str, &'a str)>,
}

/// The arguments of `{% with %}` when they are all legacy pairs, which Django requires them to be
/// once the first one is.
fn with_clause<'a>(bits: &[&'a str]) -> Vec<LegacyClause<'a>> {
    match assignments(bits, 1, |_| true) {
        Some((Assignments::Legacy(pairs), end)) if end == bits.len() => {
            vec![LegacyClause {
                bits: 1..end,
                pairs,
            }]
        }
        _ => Vec::new(),
    }
}

/// The legacy `with` and `count` clauses of `{% blocktranslate %}`, none when Django rejects it.
fn blocktranslate_clauses<'a>(bits: &[&'a str]) -> Vec<LegacyClause<'a>> {
    let mut legacy = Vec::new();
    let mut seen = Vec::new();
    let mut i = 1;
    while let Some(&option) = bits.get(i) {
        if seen.contains(&option) {
            return Vec::new();
        }
        seen.push(option);
        i += 1;
        match option {
            "with" | "count" => {
                // After a pair without `and`, Django reads an option word as the next option.
                let Some((clause, end)) =
                    assignments(bits, i, |bit| !BLOCKTRANSLATE_OPTIONS.contains(&bit))
                else {
                    return Vec::new();
                };
                if option == "count" && clause.len() != 1 {
                    return Vec::new();
                }
                if let Assignments::Legacy(pairs) = clause {
                    legacy.push(LegacyClause {
                        bits: i..end,
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

enum Assignments<'a> {
    /// `name=value` bits, already in the newer form.
    Modern(usize),
    /// `(name, value)` pairs.
    Legacy(Vec<(&'a str, &'a str)>),
}

impl Assignments<'_> {
    const fn len(&self) -> usize {
        match self {
            Self::Modern(count) => *count,
            Self::Legacy(pairs) => pairs.len(),
        }
    }
}

/// Django's `token_kwargs(bits[i:], support_legacy=True)` and the index after what it reads, or
/// [`None`] when Django finds no assignment, cannot compile a value, or a legacy name has no
/// `name=value` spelling.
///
/// A legacy pair followed by another without `and` continues the clause when `continues` accepts
/// the next bit; Django stops there.
fn assignments<'a>(
    bits: &[&'a str],
    mut i: usize,
    continues: impl Fn(&str) -> bool,
) -> Option<(Assignments<'a>, usize)> {
    // Django's `kwarg_re` decides the format on the first bit.
    if bits.get(i).is_some_and(|bit| kwarg_value(bit).is_some()) {
        let start = i;
        while let Some(value) = bits.get(i).and_then(|bit| kwarg_value(bit)) {
            if !compiles(value) {
                return None;
            }
            i += 1;
        }
        return Some((Assignments::Modern(i - start), i));
    }
    let mut pairs = Vec::new();
    while let [value, "as", name, ..] = bits[i..] {
        if !compiles(value) || !is_word(name) {
            return None;
        }
        pairs.push((name, value));
        i += 3;
        match bits.get(i) {
            Some(&"and") => i += 1,
            Some(&next) if continues(next) => {}
            _ => break,
        }
    }
    (!pairs.is_empty()).then_some((Assignments::Legacy(pairs), i))
}

/// The value of a `name=value` bit, read like Django's `kwarg_re`.
fn kwarg_value(bit: &str) -> Option<&str> {
    bit.split_once('=')
        .filter(|(name, value)| is_word(name) && !value.is_empty())
        .map(|(_, value)| value)
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

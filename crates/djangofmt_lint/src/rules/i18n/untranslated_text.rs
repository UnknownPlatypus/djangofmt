use markup_fmt::ast::{Attribute, Element, NativeAttribute, Node, NodeKind};
use miette::SourceSpan;

use crate::Checker;
use crate::checker::ContextFlags;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::contains_interpolation;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for literal text in templates that is not wrapped in a Django translation tag.
///
/// ## Why is this bad?
/// Literal text is invisible to Django's translation machinery: `makemessages` only collects
/// strings wrapped in `{% translate %}` / `{% blocktranslate %}`, so unwrapped copy stays in the
/// source language for every locale.
///
/// Text inside `<script>`, `<style>`, `<pre>` and `<textarea>` elements and inside
/// `{% blocktranslate %}`, `{% comment %}` and `{% verbatim %}` blocks is not reported, nor is
/// text without letters (numbers, punctuation, character references). Attribute values are only
/// checked for `alt`, `title`, `placeholder`, `aria-label` and `value` on submit, button or reset
/// `<input>` elements; values containing template syntax are skipped.
///
/// ## Example
/// ```html
/// <h1>Welcome</h1>
/// ```
///
/// Use instead:
/// ```html
/// <h1>{% translate "Welcome" %}</h1>
/// ```
///
/// ## Fix safety
/// This rule's fix is marked as unsafe: the template must `{% load i18n %}` for the inserted tag
/// to render, wrapping changes the strings extracted into `.po` files, and text adjacent to tags,
/// markup or interpolations may be wrapped as a sentence fragment that translates poorly.
///
/// ## References
/// - [Django documentation: Translations](https://docs.djangoproject.com/en/stable/topics/i18n/translation/)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(preview_since = "NEXT_DJANGOFMT_VERSION")]
pub struct UntranslatedText {
    source: Source,
    wrap: Option<Wrap>,
}

/// Where the untranslated text was found.
#[derive(Debug, PartialEq, Eq)]
enum Source {
    /// A text node with no adjacent interpolation.
    Standalone,
    /// A text node adjacent to a `{{ ... }}` interpolation.
    Interpolated,
    /// A translatable attribute value.
    Attribute,
}

/// Which translation tag the attached fix wraps the text in.
#[derive(Debug, PartialEq, Eq)]
enum Wrap {
    Translate,
    Blocktranslate,
}

impl Violation for UntranslatedText {
    const RULE: Rule = Rule::UntranslatedText;
    const CATEGORY: RuleCategory = RuleCategory::I18n;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    #[derive_message_formats]
    fn message(&self) -> String {
        "Found text that is not wrapped in a translation tag.".to_string()
    }

    fn help(&self) -> Option<String> {
        let help = match (&self.wrap, &self.source) {
            (Some(Wrap::Translate), _) | (None, Source::Standalone | Source::Attribute) => {
                "Wrap the text in `{% translate %}`."
            }
            (Some(Wrap::Blocktranslate), _) => {
                "Wrap the text in `{% blocktranslate trimmed %}...{% endblocktranslate %}`."
            }
            (None, Source::Interpolated) => {
                "Wrap the text and its variables in `{% blocktranslate %}...{% endblocktranslate %}`; \
                 placeholders may need manual names."
            }
        };
        Some(help.to_string())
    }

    fn fix_title(&self) -> Option<String> {
        match &self.wrap {
            Some(Wrap::Translate) => Some("Wrap in `{% translate %}`".to_string()),
            Some(Wrap::Blocktranslate) => {
                Some("Wrap in `{% blocktranslate trimmed %}`".to_string())
            }
            None => None,
        }
    }
}

/// Contexts whose text nodes are never translatable.
const SKIPPED_TEXT_CONTEXTS: ContextFlags = ContextFlags::RAW_TEXT_ELEMENT
    .union(ContextFlags::TRANSLATED_BLOCK)
    .union(ContextFlags::COMMENT_BLOCK)
    .union(ContextFlags::VERBATIM_BLOCK);

/// Attributes whose value is user-visible text on any element.
const TRANSLATABLE_ATTRS: [&str; 4] = ["alt", "title", "placeholder", "aria-label"];

/// `<input type>` values whose `value` attribute is a button label.
const BUTTON_INPUT_TYPES: [&str; 3] = ["submit", "button", "reset"];

/// Report untranslated text nodes among one group of sibling nodes.
pub fn check_text(children: &[Node<'_>], checker: &Checker<'_>) {
    if checker.flags().intersects(SKIPPED_TEXT_CONTEXTS) {
        return;
    }

    for (index, node) in children.iter().enumerate() {
        let NodeKind::Text(text) = &node.kind else {
            continue;
        };
        let trimmed = text.raw.trim();
        if !has_translatable_text(trimmed) {
            continue;
        }
        let offset = text.start + (text.raw.len() - text.raw.trim_start().len());
        let span: SourceSpan = (offset, trimmed.len()).into();

        if is_interpolation_adjacent(children, index) {
            checker.report_diagnostic(
                &UntranslatedText {
                    source: Source::Interpolated,
                    wrap: None,
                },
                span,
            );
            continue;
        }

        let (wrap, content) = wrap_body_text(trimmed);
        let mut guard = checker.report_diagnostic(
            &UntranslatedText {
                source: Source::Standalone,
                wrap: Some(wrap),
            },
            span,
        );
        guard.set_fix(Fix::unsafe_edit(Edit::replacement(content, span)));
    }
}

/// Report an untranslated value on a translatable attribute.
pub fn check_attr(attr: &NativeAttribute<'_>, element: &Element<'_>, checker: &Checker<'_>) {
    // Inside `{% verbatim %}` an inserted tag would render literally.
    if checker.flags().contains(ContextFlags::VERBATIM_BLOCK) {
        return;
    }

    let NativeAttribute {
        name,
        value: Some((value, offset)),
        quote,
    } = attr
    else {
        return;
    };

    if !is_translatable_attr(name, element) {
        return;
    }

    // `{{ }}` / `{% %}` / `{# #}` values are already dynamic.
    if contains_interpolation(value) || value.contains("{#") {
        return;
    }

    if !has_translatable_text(value) {
        return;
    }

    let span: SourceSpan = (*offset, value.len()).into();
    let fix_content = translate_attr_fix(value, *quote);
    let mut guard = checker.report_diagnostic(
        &UntranslatedText {
            source: Source::Attribute,
            wrap: fix_content.is_some().then_some(Wrap::Translate),
        },
        span,
    );
    if let Some(content) = fix_content {
        guard.set_fix(Fix::unsafe_edit(Edit::replacement(content, span)));
    }
}

/// Build the fix content wrapping a standalone text node.
fn wrap_body_text(text: &str) -> (Wrap, String) {
    // `{% translate %}` cannot hold newlines (Django tags are single-line), `%}`
    // (ends the tag early), `\` (escape processing), or both quote kinds at once.
    if !text.contains('\n')
        && !text.contains("%}")
        && !text.contains('\\')
        && let Some(quote) = available_quote(text, None)
    {
        return (
            Wrap::Translate,
            format!("{{% translate {quote}{text}{quote} %}}"),
        );
    }
    (
        Wrap::Blocktranslate,
        format!("{{% blocktranslate trimmed %}}{text}{{% endblocktranslate %}}"),
    )
}

/// Build the fix content for an attribute value, if a valid tag can hold it.
fn translate_attr_fix(value: &str, outer_quote: Option<char>) -> Option<String> {
    if value.contains('\n') || value.contains("%}") || value.contains('\\') {
        return None;
    }
    let quote = available_quote(value, Some(outer_quote?))?;
    Some(format!("{{% translate {quote}{value}{quote} %}}"))
}

/// A quote char usable around `text` inside a Django tag, excluding `exclude`.
fn available_quote(text: &str, exclude: Option<char>) -> Option<char> {
    ['"', '\'']
        .into_iter()
        .find(|&quote| Some(quote) != exclude && !text.contains(quote))
}

fn is_translatable_attr(name: &str, element: &Element<'_>) -> bool {
    if TRANSLATABLE_ATTRS
        .iter()
        .any(|attr| name.eq_ignore_ascii_case(attr))
    {
        return true;
    }
    name.eq_ignore_ascii_case("value")
        && element.tag_name.eq_ignore_ascii_case("input")
        && element.attrs.iter().any(is_button_type_attr)
}

/// Whether `attr` is a literal `type` attribute naming a button-like input.
fn is_button_type_attr(attr: &Attribute<'_>) -> bool {
    let Attribute::Native(NativeAttribute {
        name,
        value: Some((value, _)),
        ..
    }) = attr
    else {
        return false;
    };
    name.eq_ignore_ascii_case("type")
        && BUTTON_INPUT_TYPES
            .iter()
            .any(|ty| value.eq_ignore_ascii_case(ty))
}

/// Whether the nearest rendering sibling on either side is an interpolation.
fn is_interpolation_adjacent(children: &[Node<'_>], index: usize) -> bool {
    let is_interpolation = |node: &Node<'_>| matches!(node.kind, NodeKind::JinjaInterpolation(_));
    children[..index]
        .iter()
        .rev()
        .find(|node| !is_transparent(node))
        .is_some_and(is_interpolation)
        || children[index + 1..]
            .iter()
            .find(|node| !is_transparent(node))
            .is_some_and(is_interpolation)
}

/// Whether `node` is skipped when looking for the nearest rendering sibling.
fn is_transparent(node: &Node<'_>) -> bool {
    match &node.kind {
        NodeKind::Comment(_) | NodeKind::JinjaComment(_) => true,
        NodeKind::Text(text) => text.raw.chars().all(char::is_whitespace),
        _ => false,
    }
}

/// Whether `text` contains at least one letter outside HTML character
/// references (`&name;`, `&#123;`, `&#xAB;`).
fn has_translatable_text(text: &str) -> bool {
    let mut rest = text;
    while let Some(pos) = rest.find(|c: char| c.is_alphabetic() || c == '&') {
        let tail = &rest[pos..];
        let Some(after_amp) = tail.strip_prefix('&') else {
            return true;
        };
        rest = char_reference_len(after_amp).map_or(after_amp, |len| &after_amp[len..]);
    }
    false
}

/// Length of a character-reference body (`name;`, `#123;`, `#xAB;`) at the start of `s`.
fn char_reference_len(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let (start, is_body_byte): (usize, fn(u8) -> bool) = if bytes.first() == Some(&b'#') {
        if matches!(bytes.get(1), Some(b'x' | b'X')) {
            (2, |b| b.is_ascii_hexdigit())
        } else {
            (1, |b| b.is_ascii_digit())
        }
    } else {
        (0, |b| b.is_ascii_alphanumeric())
    };
    let body_len = bytes[start..]
        .iter()
        .take_while(|&&b| is_body_byte(b))
        .count();
    (body_len > 0 && bytes.get(start + body_len) == Some(&b';')).then_some(start + body_len + 1)
}

use std::borrow::Cow;

use djangofmt_syntax::dtl::{bits, first_bit_is, is_space};
use markup_fmt::ast::{JinjaBlock, JinjaTag, JinjaTagOrChildren};

use crate::Checker;
use crate::checker::TagOrigin;
use crate::django_version::DjangoVersion;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::rules::helpers::tags_in_text;
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for the `{% trans %}` and `{% blocktrans %}` template tags, the names Django gave
/// `{% translate %}` and `{% blocktranslate %}` before version 3.1.
///
/// ## Why is this bad?
/// Django 3.1 renamed both tags. The old names remain as aliases, retained for the foreseeable
/// future, but the documentation only uses the new ones, and a template that mixes both spellings
/// reads inconsistently. `makemessages` extracts strings from either spelling, so the rename leaves
/// translation catalogs unchanged.
///
/// The fix renames each `{% endblocktrans %}` along with its opening tag, since Django requires the
/// two to match, and updates `{% load trans blocktrans from i18n %}` to import the new names.
///
/// The rule is version-gated: it reports nothing until `lint.target-version` names Django 3.1 or
/// newer.
///
/// ## Example
/// ```html
/// {% load i18n %}
/// <h1>{% trans "Welcome" %}</h1>
/// <p>{% blocktrans trimmed %}Hello {{ name }}{% endblocktrans %}</p>
/// ```
///
/// Use instead:
/// ```html
/// {% load i18n %}
/// <h1>{% translate "Welcome" %}</h1>
/// <p>{% blocktranslate trimmed %}Hello {{ name }}{% endblocktranslate %}</p>
/// ```
///
/// ## Options
/// - `lint.target-version`
///
/// ## References
/// - [Django 3.1 release notes: templates](https://docs.djangoproject.com/en/stable/releases/3.1/#templates)
/// - [Django documentation: `translate`](https://docs.djangoproject.com/en/stable/topics/i18n/translation/#std-templatetag-translate)
/// - [Django documentation: `blocktranslate`](https://docs.djangoproject.com/en/stable/topics/i18n/translation/#std-templatetag-blocktranslate)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct LegacyTranslationTag {
    pub tag: LegacyTag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyTag {
    Trans,
    Blocktrans,
}

impl Violation for LegacyTranslationTag {
    const RULE: Rule = Rule::LegacyTranslationTag;
    const CATEGORY: RuleCategory = RuleCategory::Upgrade;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        match self.tag {
            LegacyTag::Trans => "Legacy `{% trans %}` tag name".into(),
            LegacyTag::Blocktrans => "Legacy `{% blocktrans %}` tag name".into(),
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some(match self.tag {
            LegacyTag::Trans => "Use `{% translate %}`".into(),
            LegacyTag::Blocktrans => "Use `{% blocktranslate %}`".into(),
        })
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Use the tag's newer name")
    }
}

/// The release that added `translate` and `blocktranslate`.
const RENAMED_IN: DjangoVersion = DjangoVersion::new(3, 1);

/// Every `{% %}`: `{% trans %}`, `{% load … from i18n %}`, and a `{% blocktrans %}` in text, whose
/// closer the parser never pairs. The caller passes the target version, so files without one never
/// get here.
pub fn check_tag(
    checker: &Checker<'_>,
    tag: &JinjaTag<'_>,
    origin: TagOrigin,
    target_version: DjangoVersion,
) {
    if !checker.is_django() || target_version < RENAMED_IN {
        return;
    }
    // Every tag gets here, so a first byte that starts no candidate name, nor rarer whitespace than
    // ASCII, rejects it before the rest runs out of line.
    let first = tag.content.trim_ascii_start().as_bytes().first();
    if matches!(
        first,
        Some(b't' | b'l' | b'\x0B' | b'\x1C'..=b'\x1F' | 0x80..)
    ) || (first == Some(&b'b') && origin == TagOrigin::Text)
    {
        check_name(checker, tag, origin);
    }
}

#[cold]
fn check_name(checker: &Checker<'_>, tag: &JinjaTag<'_>, origin: TagOrigin) {
    if first_bit_is(tag.content, "trans") {
        report(checker, LegacyTag::Trans, tag.content);
    } else if first_bit_is(tag.content, "load") {
        check_load(checker, tag.content);
    } else if origin == TagOrigin::Text && first_bit_is(tag.content, "blocktrans") {
        check_text_pair(checker, tag);
    }
}

/// A `{% blocktrans %}` block of the AST, from its opener to its closer.
pub fn check_block<T>(
    checker: &Checker<'_>,
    block: &JinjaBlock<'_, T>,
    target_version: DjangoVersion,
) {
    if !checker.is_django() || target_version < RENAMED_IN {
        return;
    }
    // The parser closes a block only with `end` plus its name, so the closer is `endblocktrans`.
    if let [
        JinjaTagOrChildren::Tag(opener),
        ..,
        JinjaTagOrChildren::Tag(closer),
    ] = &block.body[..]
        && first_bit_is(opener.content, "blocktrans")
    {
        report_pair(checker, opener.content, closer.content);
    }
}

fn check_load(checker: &Checker<'_>, content: &str) {
    // Only `{% load … from i18n %}` imports names from `i18n`, so the other loads are not lexed.
    let Some(rest) = content.trim_end_matches(is_space).strip_suffix("i18n") else {
        return;
    };
    if !rest.trim_end_matches(is_space).ends_with("from") {
        return;
    }
    // Django reads the `from` form only from four bits on, which leaves `names` empty below that.
    if let [_, names @ .., "from", "i18n"] = &bits(content)[..] {
        for &name in names {
            match name {
                "trans" => report(checker, LegacyTag::Trans, name),
                "blocktrans" => report(checker, LegacyTag::Blocktrans, name),
                _ => {}
            }
        }
    }
}

/// Django closes a `{% blocktrans %}` with the next tag, or the one after a `{% plural %}`, which
/// may lie past the end of the opener's text. So the closer is found the way Django's lexer finds
/// it, in the rest of the source.
fn check_text_pair(checker: &Checker<'_>, opener: &JinjaTag<'_>) {
    let after_opener = checker.source_end(opener.content) + "%}".len();
    let mut next_tags = tags_in_text(&checker.context().source()[after_opener..])
        .map(|tag| tag.content.trim_matches(is_space));
    let mut closer = next_tags.next();
    if closer == Some("plural") {
        closer = next_tags.next();
    }
    if let Some(closer @ "endblocktrans") = closer {
        report_pair(checker, opener.content, closer);
    }
}

/// The `name` that `content` starts with, which the caller checked with [`first_bit_is`], as a
/// slice of `content` so that it locates itself.
fn first_bit<'a>(content: &'a str, name: &str) -> &'a str {
    let start = content.len() - content.trim_start_matches(is_space).len();
    &content[start..start + name.len()]
}

/// Rename the legacy name `content` starts with.
#[cold]
fn report(checker: &Checker<'_>, tag: LegacyTag, content: &str) {
    let (name, new_name) = match tag {
        LegacyTag::Trans => ("trans", "translate"),
        LegacyTag::Blocktrans => ("blocktrans", "blocktranslate"),
    };
    let span = checker.source_span(first_bit(content, name));
    checker
        .report_diagnostic(&LegacyTranslationTag { tag }, span)
        .set_fix(Fix::safe_edit(Edit::replacement(new_name, span)));
}

/// One fix for both tags: Django rejects a `{% blocktranslate %}` closed by `{% endblocktrans %}`.
#[cold]
fn report_pair(checker: &Checker<'_>, opener: &str, closer: &str) {
    let span = checker.source_span(first_bit(opener, "blocktrans"));
    let closer = checker.source_span(first_bit(closer, "endblocktrans"));
    checker
        .report_diagnostic(
            &LegacyTranslationTag {
                tag: LegacyTag::Blocktrans,
            },
            span,
        )
        .set_fix(Fix::safe_edits(
            Edit::replacement("blocktranslate", span),
            [Edit::replacement("endblocktranslate", closer)],
        ));
}

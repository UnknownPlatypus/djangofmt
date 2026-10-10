use std::borrow::Cow;

use djangofmt_syntax::dtl::{bits, is_space, tag_name};
use markup_fmt::ast::JinjaTag;

use crate::Checker;
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
/// two to match, and updates `{% load trans blocktrans from i18n %}` to import the new names. Such
/// a load and the tags using the names it imports get separate fixes, so ignoring the rule on some
/// of those lines but not the others leaves a template Django rejects. The tags after a load that
/// takes `trans` or `blocktrans` from another library, such as `{% load trans from mylib %}`, are
/// left alone, since they may be that library's.
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

impl LegacyTag {
    const fn new_name(self) -> &'static str {
        match self {
            Self::Trans => "translate",
            Self::Blocktrans => "blocktranslate",
        }
    }
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

/// Whether the rule applies: Django only, from the release that added the new names.
fn applies(checker: &Checker<'_>, target_version: DjangoVersion) -> bool {
    checker.is_django() && target_version >= RENAMED_IN
}

/// The caller guarantees a `{% trans %}` that no earlier load took from another library, and
/// passes the target version, so files without one never get here.
pub fn check_trans(checker: &Checker<'_>, tag: &JinjaTag<'_>, target_version: DjangoVersion) {
    if applies(checker, target_version) {
        report(checker, LegacyTag::Trans, tag_name(tag.content), None);
    }
}

/// The caller guarantees a `{% blocktrans %}` opener, parsed or found in text, that no earlier
/// load took from another library. Django closes it with the next tag, or the one after a
/// `{% plural %}`, which may lie past the end of the opener's text: the closer is found the way
/// Django's lexer finds it, in the rest of the source.
pub fn check_blocktrans(checker: &Checker<'_>, tag: &JinjaTag<'_>, target_version: DjangoVersion) {
    if !applies(checker, target_version) {
        return;
    }
    let after_opener = checker.source_end(tag.content) + "%}".len();
    let mut next_tags = tags_in_text(&checker.context().source()[after_opener..])
        .map(|next| next.content.trim_matches(is_space));
    let mut closer = next_tags.next();
    if closer == Some("plural") {
        closer = next_tags.next();
    }
    if let Some(closer @ "endblocktrans") = closer {
        report(
            checker,
            LegacyTag::Blocktrans,
            tag_name(tag.content),
            Some(closer),
        );
    }
}

/// The caller guarantees a `{% load %}`. Reports the legacy names it imports from `i18n`, and
/// returns whether it imports one from another library, whose tag then goes by that name.
pub fn check_load(
    checker: &Checker<'_>,
    tag: &JinjaTag<'_>,
    target_version: DjangoVersion,
) -> bool {
    if !applies(checker, target_version) || !tag.content.contains("trans") {
        return false;
    }
    // Django reads the `from` form only from four bits on, which leaves `names` empty below that.
    let [_, names @ .., "from", library] = &bits(tag.content)[..] else {
        return false;
    };
    if *library != "i18n" {
        return names
            .iter()
            .any(|name| matches!(*name, "trans" | "blocktrans"));
    }
    for &name in names {
        match name {
            "trans" => report(checker, LegacyTag::Trans, name, None),
            "blocktrans" => report(checker, LegacyTag::Blocktrans, name, None),
            _ => {}
        }
    }
    false
}

/// Rename `name`, a slice of the source, and the pair's `closer` with it: Django rejects a
/// `{% blocktranslate %}` closed by `{% endblocktrans %}`.
#[cold]
fn report(checker: &Checker<'_>, tag: LegacyTag, name: &str, closer: Option<&str>) {
    let span = checker.source_span(name);
    let rename = Edit::replacement(tag.new_name(), span);
    let fix = match closer {
        Some(closer) => Fix::safe_edits(
            rename,
            [Edit::replacement(
                "endblocktranslate",
                checker.source_span(closer),
            )],
        ),
        None => Fix::safe_edit(rename),
    };
    checker
        .report_diagnostic(&LegacyTranslationTag { tag }, span)
        .set_fix(fix);
}

use std::borrow::Cow;

use djangofmt_syntax::dtl::bits;
use djangofmt_syntax::span;
use markup_fmt::ast::JinjaTag;

use crate::Checker;
use crate::django_version::DjangoVersion;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};

/// ## What it does
/// Checks for `{% load %}` tags that load the `staticfiles` or `admin_static` template libraries.
///
/// ## Why is this bad?
/// Both libraries are aliases of `static`, deprecated in Django 2.1 and removed in Django 3.0.
/// Loading them gains nothing over `static`, and fails outright from Django 3.0 on.
///
/// The rule is version-gated: it reports nothing until `lint.target-version` names Django 2.1 or
/// newer.
///
/// ## Example
/// ```html
/// {% load staticfiles %}
/// <img src="{% static 'logo.png' %}" alt="Logo" width="64" height="64">
/// ```
///
/// Use instead:
/// ```html
/// {% load static %}
/// <img src="{% static 'logo.png' %}" alt="Logo" width="64" height="64">
/// ```
///
/// ## Options
/// - `lint.target-version`
///
/// ## References
/// - [Django 2.1 release notes: deprecated features](https://docs.djangoproject.com/en/stable/releases/2.1/#deprecated-features-2-1)
/// - [Django 3.0 release notes: removed features](https://docs.djangoproject.com/en/stable/releases/3.0/#features-removed-in-3-0)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct DeprecatedStaticLibrary<'a> {
    pub library: &'a str,
}

impl Violation for DeprecatedStaticLibrary<'_> {
    const RULE: Rule = Rule::DeprecatedStaticLibrary;
    const CATEGORY: RuleCategory = RuleCategory::Upgrade;
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> Cow<'static, str> {
        format!("Deprecated template library `{}`", self.library).into()
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        Some("Load `static` instead".into())
    }

    fn fix_title(&self) -> Option<&'static str> {
        Some("Load `static` instead")
    }
}

/// The release that deprecated both libraries in favour of `static`.
const DEPRECATED_IN: DjangoVersion = DjangoVersion::new(2, 1);

const DEPRECATED_LIBRARIES: [&str; 2] = ["staticfiles", "admin_static"];

/// The caller guarantees the tag is a `load` and passes the target version, so files without
/// one never get here.
pub fn check(checker: &Checker<'_>, tag: &JinjaTag<'_>, target_version: DjangoVersion) {
    if checker.is_django()
        && target_version >= DEPRECATED_IN
        && DEPRECATED_LIBRARIES
            .iter()
            .any(|library| tag.content.contains(library))
    {
        check_libraries(checker, tag.content);
    }
}

#[cold]
fn check_libraries(checker: &Checker<'_>, content: &str) {
    let bits = bits(content);
    match bits[..] {
        // `static` is the only tag either alias defines, so a `from` load of anything else is left alone.
        [_, kept @ "static", "from", library] if DEPRECATED_LIBRARIES.contains(&library) => {
            report(checker, library, delete_after(checker, kept, library));
        }
        // Django reads `{% load a b from library %}` only from four bits on.
        [_, _, .., "from", _] => {}
        _ => {
            // Renaming an alias once the tag loads `static` would load it twice, so drop the alias.
            let mut loads_static = bits.contains(&"static");
            for (previous, &library) in bits.iter().zip(&bits[1..]) {
                if !DEPRECATED_LIBRARIES.contains(&library) {
                    continue;
                }
                let edit = if loads_static {
                    delete_after(checker, previous, library)
                } else {
                    Edit::replacement("static", checker.source_span(library))
                };
                report(checker, library, edit);
                loads_static = true;
            }
        }
    }
}

/// Delete `library` along with the whitespace before it, from the end of the `kept` bit.
fn delete_after(checker: &Checker<'_>, kept: &str, library: &str) -> Edit {
    let start = checker.source_end(kept);
    Edit::deletion(span(start, checker.source_end(library) - start))
}

fn report(checker: &Checker<'_>, library: &str, edit: Edit) {
    checker
        .report_diagnostic(
            &DeprecatedStaticLibrary { library },
            checker.source_span(library),
        )
        .set_fix(Fix::safe_edit(edit));
}

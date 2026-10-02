use std::borrow::Cow;

use djangofmt_syntax::dtl::{bits, first_bit_is};
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

/// The caller passes the target version, so files without one never get here.
pub fn check(checker: &Checker<'_>, tag: &JinjaTag<'_>, target_version: DjangoVersion) {
    if !checker.is_django() || target_version < DEPRECATED_IN {
        return;
    }
    // Every tag gets here, so the name is matched without lexing and the rest runs out of line.
    if first_bit_is(tag.content, "load") {
        check_load(checker, tag.content);
    }
}

#[cold]
fn check_load(checker: &Checker<'_>, content: &str) {
    // Both libraries contain `static`, so the other `{% load %}` tags are not lexed.
    if !content.contains("static") {
        return;
    }
    let bits = bits(content);
    // Django reads `{% load a b from library %}` only from four bits on.
    if bits.len() >= 4 && bits[bits.len() - 2] == "from" {
        // `static` is the only tag either alias defines, so anything else is already broken.
        if let ["load", "static", "from", library] = bits[..]
            && DEPRECATED_LIBRARIES.contains(&library)
        {
            let start = checker.source_end(bits[1]);
            let edit = Edit::deletion(span(start, checker.source_end(library) - start));
            report(checker, library, edit);
        }
        return;
    }

    // Renaming an alias once the tag loads `static` would load it twice, so drop the alias.
    let mut loads_static = bits.contains(&"static");
    for (previous, &library) in bits.iter().zip(&bits[1..]) {
        if !DEPRECATED_LIBRARIES.contains(&library) {
            continue;
        }
        let edit = if loads_static {
            let start = checker.source_end(previous);
            Edit::deletion(span(start, checker.source_end(library) - start))
        } else {
            Edit::replacement("static", checker.source_span(library))
        };
        report(checker, library, edit);
        loads_static = true;
    }
}

fn report(checker: &Checker<'_>, library: &str, edit: Edit) {
    checker
        .report_diagnostic(
            &DeprecatedStaticLibrary { library },
            checker.source_span(library),
        )
        .set_fix(Fix::safe_edit(edit));
}

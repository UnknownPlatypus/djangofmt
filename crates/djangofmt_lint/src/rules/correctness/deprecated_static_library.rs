use std::borrow::Cow;

use markup_fmt::ast::JinjaTag;
use markup_fmt::parser::parse_jinja_tag_name;
use miette::SourceSpan;

use crate::django_version::DjangoVersion;
use crate::fix::{Edit, Fix, FixAvailability};
use crate::registry::{Rule, RuleCategory};
use crate::violation::{Violation, ViolationMetadata, derive_message_formats};
use crate::{Checker, span};

/// ## What it does
/// Checks for `{% load %}` tags that import the `staticfiles` or `admin_static` template libraries.
///
/// ## Why is this bad?
/// Both libraries are aliases of `static`, deprecated in Django 2.1 and removed in Django 3.0.
/// They mark the template as one that breaks on upgrade while offering nothing `static` does not
/// already provide.
///
/// The rule is version-gated: it reports nothing until `lint.target-version` names Django 2.1
/// or newer.
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
/// - [Django 2.1 release notes](https://docs.djangoproject.com/en/stable/releases/2.1/#deprecated-features-2-1)
/// - [Django 3.0 release notes](https://docs.djangoproject.com/en/stable/releases/3.0/#features-removed-in-3-0)
#[derive(Debug, PartialEq, Eq, ViolationMetadata)]
#[violation_metadata(stable_since = "NEXT_DJANGOFMT_VERSION")]
pub struct DeprecatedStaticLibrary<'a> {
    pub library: &'a str,
}

impl Violation for DeprecatedStaticLibrary<'_> {
    const RULE: Rule = Rule::DeprecatedStaticLibrary;
    const CATEGORY: RuleCategory = RuleCategory::Correctness;
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

const STATIC: &str = "static";
const DEPRECATED_LIBRARIES: [&str; 2] = ["staticfiles", "admin_static"];

pub fn check(tag: &JinjaTag<'_>, checker: &Checker<'_>) {
    if !checker.targets_django(DEPRECATED_IN) {
        return;
    }

    let tag_name = parse_jinja_tag_name(tag);
    if tag_name != "load" {
        return;
    }
    let Some(args) = tag.content.trim_start().strip_prefix(tag_name) else {
        return;
    };

    let libraries = library_arguments(args);
    // Once the tag loads `static` -- already, or because an earlier alias was renamed to it -- a
    // further alias is redundant, and renaming it too would emit `{% load static static %}`.
    let mut loads_static = libraries.split_ascii_whitespace().any(|lib| lib == STATIC);

    for library in libraries.split_ascii_whitespace() {
        if !DEPRECATED_LIBRARIES.contains(&library) {
            continue;
        }

        let library_span = checker.source_span(library);
        let mut guard =
            checker.report_diagnostic(&DeprecatedStaticLibrary { library }, library_span);
        guard.set_fix(Fix::safe_edit(if loads_static {
            Edit::deletion(span_with_separator(library, args, checker))
        } else {
            Edit::replacement(STATIC, library_span)
        }));
        loads_static = true;
    }
}

/// The slice of a `{% load %}` tag's arguments that names libraries.
///
/// `{% load a b from lib %}` imports only `lib`; every other form imports each argument.
fn library_arguments(args: &str) -> &str {
    let mut tokens = args.split_ascii_whitespace().rev();
    match (tokens.next(), tokens.next()) {
        (Some(library), Some("from")) => library,
        _ => args,
    }
}

/// `token`'s span widened over the whitespace separating it from the previous argument, so
/// deleting a library leaves the rest single-spaced. `{% load %}` always separates its tag name
/// from the first argument, so there is always whitespace to absorb.
fn span_with_separator(token: &str, args: &str, checker: &Checker<'_>) -> SourceSpan {
    let args_start = checker.source_offset(args);
    let before_token = &args[..checker.source_offset(token) - args_start];
    let start = args_start + before_token.trim_end().len();
    span(start, checker.source_end(token) - start)
}

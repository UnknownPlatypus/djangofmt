//! Inference of the targeted Django version from the `django` requirement of a project.

use djangofmt_lint::DjangoVersion;

/// The oldest Django version the project may run on, from its `django` requirements.
pub fn infer_target_version<'a>(
    requirements: impl IntoIterator<Item = &'a str>,
) -> Option<DjangoVersion> {
    requirements
        .into_iter()
        .filter_map(django_lower_bound)
        .min()
}

/// Lower bound of a PEP 508 requirement when it names `django`.
fn django_lower_bound(requirement: &str) -> Option<DjangoVersion> {
    let requirement = requirement.trim();
    let name_end = requirement
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')))
        .unwrap_or(requirement.len());
    if !requirement[..name_end].eq_ignore_ascii_case("django") {
        return None;
    }

    let mut rest = requirement[name_end..].trim_start();
    if let Some(after_bracket) = rest.strip_prefix('[') {
        rest = after_bracket.split_once(']')?.1.trim_start();
    }
    // Drop the environment marker, then refuse URL requirements (`django @ git+...`).
    let specifiers = rest
        .split_once(';')
        .map_or(rest, |(before, _)| before)
        .trim();
    if specifiers.starts_with('@') {
        return None;
    }

    specifiers
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split(',')
        .filter_map(specifier_lower_bound)
        .min()
}

/// Lower bound of a single PEP 440 specifier. Upper bounds and exclusions bound nothing.
fn specifier_lower_bound(specifier: &str) -> Option<DjangoVersion> {
    let specifier = specifier.trim();
    let operator_end = specifier
        .find(|c: char| !matches!(c, '=' | '~' | '>' | '<' | '!'))
        .unwrap_or(specifier.len());
    let (operator, version) = specifier.split_at(operator_end);
    if !matches!(operator, "==" | "===" | "~=" | ">=" | ">") {
        return None;
    }

    let version = version.trim();
    // An epoch (`1!4.2`) only reorders releases against each other.
    let version = version.rsplit_once('!').map_or(version, |(_, rest)| rest);
    let mut segments = version.split('.');
    let major = leading_number(segments.next()?)?;
    let minor = segments.next().and_then(leading_number).unwrap_or(0);
    Some(DjangoVersion::new(major, minor))
}

/// Digits at the start of a release segment, so `2a1`, `2rc1`, `dev1` and `*` stop early.
fn leading_number(segment: &str) -> Option<u8> {
    let end = segment
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(segment.len());
    segment[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{django_lower_bound, infer_target_version};
    use djangofmt_lint::DjangoVersion;
    use rstest::rstest;

    #[rstest]
    #[case("django>=4.2", Some(DjangoVersion::new(4, 2)))]
    #[case("Django==5.1.0", Some(DjangoVersion::new(5, 1)))]
    #[case("django~=4.1", Some(DjangoVersion::new(4, 1)))]
    #[case("django >= 4.2.1", Some(DjangoVersion::new(4, 2)))]
    #[case("django>=4.2,<5.0", Some(DjangoVersion::new(4, 2)))]
    #[case("django[argon2]>=6,<6.1", Some(DjangoVersion::new(6, 0)))]
    #[case("django==5.2.*", Some(DjangoVersion::new(5, 2)))]
    #[case("django==5.*", Some(DjangoVersion::new(5, 0)))]
    #[case(
        "django>=5.0; python_version >= \"3.10\"",
        Some(DjangoVersion::new(5, 0))
    )]
    #[case("Django>=4.2a1", Some(DjangoVersion::new(4, 2)))]
    #[case("django>=4.2.dev1", Some(DjangoVersion::new(4, 2)))]
    // The smallest release satisfying a strict lower bound is still a 4.2.x.
    #[case("django>4.2", Some(DjangoVersion::new(4, 2)))]
    #[case("django (>=4.2)", Some(DjangoVersion::new(4, 2)))]
    #[case("django>=4.2,!=4.2.1", Some(DjangoVersion::new(4, 2)))]
    #[case("django>=4.2 ; extra == \"a,b\"", Some(DjangoVersion::new(4, 2)))]
    #[case("DJANGO[a,b] ~= 5.0.3", Some(DjangoVersion::new(5, 0)))]
    #[case("django>=1!4.2", Some(DjangoVersion::new(4, 2)))]
    #[case("django", None)]
    #[case("django<6", None)]
    #[case("django @ git+https://github.com/django/django", None)]
    #[case("django-debug-toolbar>=4", None)]
    #[case("django_extensions>=3.0", None)]
    #[case("requests>=2.0", None)]
    fn lower_bound_of_a_requirement(
        #[case] requirement: &str,
        #[case] expected: Option<DjangoVersion>,
    ) {
        assert_eq!(django_lower_bound(requirement), expected);
    }

    #[test]
    fn marker_split_requirements_keep_the_oldest_version() {
        assert_eq!(
            infer_target_version([
                "requests>=2",
                "django>=5.0; python_version>='3.10'",
                "django>=4.2; python_version<'3.10'",
            ]),
            Some(DjangoVersion::new(4, 2))
        );
    }
}

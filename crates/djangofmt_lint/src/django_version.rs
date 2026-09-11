//! The Django version templates target, used to gate version-specific rules.

use std::fmt;
use std::str::FromStr;

/// A `major.minor` Django release, e.g. `5.2`.
///
/// Any `major.minor` pair is accepted so a new Django release needs no code change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DjangoVersion {
    pub major: u8,
    pub minor: u8,
}

impl DjangoVersion {
    #[must_use]
    pub const fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }
}

impl fmt::Display for DjangoVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl FromStr for DjangoVersion {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let invalid = || format!("target-version must be a `major.minor` version (got `{value}`)");
        let (major, minor) = value.split_once('.').ok_or_else(invalid)?;
        let (Ok(major), Ok(minor)) = (major.parse(), minor.parse()) else {
            return Err(invalid());
        };
        Ok(Self::new(major, minor))
    }
}

/// Deserialize from the string form, reusing [`FromStr`] so `pyproject.toml` and the CLI
/// accept exactly the same syntax.
impl<'de> serde::Deserialize<'de> for DjangoVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        value.trim().parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::DjangoVersion;
    use rstest::rstest;

    #[rstest]
    #[case("5.2", DjangoVersion::new(5, 2))]
    #[case("4.10", DjangoVersion::new(4, 10))]
    fn parses_major_minor(#[case] input: &str, #[case] expected: DjangoVersion) {
        assert_eq!(input.parse::<DjangoVersion>(), Ok(expected));
        assert_eq!(expected.to_string(), input);
    }

    #[rstest]
    #[case("5")]
    #[case("5.2.1")]
    #[case("v5.2")]
    #[case("five.two")]
    #[case("5.")]
    #[case("")]
    fn rejects_anything_else(#[case] input: &str) {
        assert_eq!(
            input.parse::<DjangoVersion>(),
            Err(format!(
                "target-version must be a `major.minor` version (got `{input}`)"
            ))
        );
    }

    #[test]
    fn orders_by_major_then_minor() {
        assert!(DjangoVersion::new(4, 2) < DjangoVersion::new(5, 0));
        assert!(DjangoVersion::new(3, 2) < DjangoVersion::new(3, 10));
    }
}

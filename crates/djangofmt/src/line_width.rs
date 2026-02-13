use serde::Deserialize;
use std::num::{NonZeroU8, NonZeroU16};
use std::str::FromStr;

/// The length of a line of text that is considered too long.
/// The allowed range of values is 1..=320.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineLength(NonZeroU16);

impl LineLength {
    const MAX: u16 = 320;

    #[must_use]
    pub const fn value(self) -> u16 {
        self.0.get()
    }
}

impl Default for LineLength {
    fn default() -> Self {
        Self(NonZeroU16::new(120).expect("120 is not zero"))
    }
}

impl TryFrom<u16> for LineLength {
    type Error = String;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match NonZeroU16::try_from(value) {
            Ok(v) if v.get() <= Self::MAX => Ok(Self(v)),
            _ => Err(format!(
                "line-length must be between 1 and {} (got {value})",
                Self::MAX
            )),
        }
    }
}

impl From<LineLength> for usize {
    fn from(value: LineLength) -> Self {
        value.0.get() as Self
    }
}

impl FromStr for LineLength {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: u16 = s
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        Self::try_from(value)
    }
}

impl<'de> Deserialize<'de> for LineLength {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u16::deserialize(deserializer)?;
        Self::try_from(value).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for LineLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// The width of an indentation level.
/// The allowed range of values is 1..=16.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndentWidth(NonZeroU8);

impl IndentWidth {
    const MAX: u8 = 16;

    #[must_use]
    pub const fn value(self) -> u8 {
        self.0.get()
    }
}

impl Default for IndentWidth {
    fn default() -> Self {
        Self(NonZeroU8::new(4).expect("4 is not zero"))
    }
}

impl TryFrom<u8> for IndentWidth {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match NonZeroU8::try_from(value) {
            Ok(v) if v.get() <= Self::MAX => Ok(Self(v)),
            _ => Err(format!(
                "indent-width must be between 1 and {} (got {value})",
                Self::MAX
            )),
        }
    }
}

impl From<IndentWidth> for usize {
    fn from(value: IndentWidth) -> Self {
        value.0.get() as Self
    }
}

impl FromStr for IndentWidth {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: u8 = s
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        Self::try_from(value)
    }
}

impl<'de> Deserialize<'de> for IndentWidth {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u8::deserialize(deserializer)?;
        Self::try_from(value).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for IndentWidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

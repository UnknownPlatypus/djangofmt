use std::path::Path;

use markup_fmt::Language;
use serde::Deserialize;

#[derive(Copy, Clone, Debug, Deserialize, Default, PartialEq, Eq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    #[default]
    Django,
    Jinja,
}

impl Profile {
    /// Infer the profile from a file's extension.
    ///
    /// - `.html` → `Django`
    /// - `.jinja`, `.jinja2`, `.j2` → `Jinja`
    ///
    /// Returns `None` for unrecognised extensions.
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "html" => Some(Self::Django),
            "jinja" | "jinja2" | "j2" => Some(Self::Jinja),
            _ => None,
        }
    }
}

impl From<Profile> for Language {
    fn from(profile: Profile) -> Self {
        match profile {
            Profile::Django => Self::Django,
            Profile::Jinja => Self::Jinja,
        }
    }
}

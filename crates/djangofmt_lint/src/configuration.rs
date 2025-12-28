use rustc_hash::FxHashSet;
use serde::Deserialize;
use std::str::FromStr;
use strum::IntoEnumIterator;

use crate::registry::{RuleCategory, RuleCode};
use crate::settings::Settings;

/// Represents a user's selection request.
#[derive(Debug, Clone)]
pub enum RuleSelector {
    Specific(RuleCode),
    Category(RuleCategory),
}

impl FromStr for RuleSelector {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Try parsing as a specific rule code first
        if let Ok(code) = RuleCode::from_str(s) {
            return Ok(Self::Specific(code));
        }

        // Try parsing as a category
        if let Ok(category) = RuleCategory::from_str(s) {
            return Ok(Self::Category(category));
        }

        Err(format!("Unknown rule or category: '{s}'"))
    }
}

/// User configuration for the linter.
/// This struct is designed to be deserialized from a configuration file (e.g., TOML).
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UserConfig {
    /// List of rules or categories to enable.
    /// If provided, this overrides the default set of rules.
    pub select: Option<Vec<String>>,

    /// List of rules or categories to ignore.
    pub ignore: Option<Vec<String>>,

    /// List of rules or categories to enable in addition to the defaults (or `select`).
    pub extend_select: Option<Vec<String>>,
}

impl UserConfig {
    /// Resolves the configuration into a final set of enabled rules.
    #[must_use]
    pub fn resolve(&self) -> Settings {
        let mut rules = FxHashSet::default();

        // 1. Determine base set
        if let Some(select) = &self.select {
            // If 'select' is present, it defines the base set
            for s in select {
                apply_selector(&mut rules, s, true);
            }
        } else {
            // If 'select' is missing, use defaults.
            // User requested default to be ALL rules.
            for rule in RuleCode::iter() {
                rules.insert(rule);
            }
        }

        // 2. Process 'extend_select'
        if let Some(extends) = &self.extend_select {
            for s in extends {
                apply_selector(&mut rules, s, true);
            }
        }

        // 3. Process 'ignore'
        if let Some(ignores) = &self.ignore {
            for s in ignores {
                apply_selector(&mut rules, s, false);
            }
        }

        Settings { rules }
    }
}

fn apply_selector(rules: &mut FxHashSet<RuleCode>, s: &str, add: bool) {
    match RuleSelector::from_str(s) {
        Ok(RuleSelector::Specific(code)) => {
            if add {
                rules.insert(code);
            } else {
                rules.remove(&code);
            }
        }
        Ok(RuleSelector::Category(category)) => {
            // Expand category to all rules
            for rule in RuleCode::iter() {
                if rule.category() == category {
                    if add {
                        rules.insert(rule);
                    } else {
                        rules.remove(&rule);
                    }
                }
            }
        }
        Err(e) => {
            // For now, just print a warning. In a real app, we might want to collect warnings.
            eprintln!("Warning: {e}");
        }
    }
}

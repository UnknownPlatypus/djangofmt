//! Structured metadata for the `[tool.djangofmt]` options.
//!
//! `#[derive(OptionsMetadata)]` records each option's doc comment, default, type and example so
//! `djangofmt_dev` can generate `docs/settings.md` and validate the `## Options` bullets of rule
//! documentation, instead of both being hand-maintained.

/// Visits the options recorded by an [`OptionsMetadata`] implementation.
pub trait Visit {
    /// Visits a single option named `name`.
    fn record_field(&mut self, name: &str, field: OptionField);

    /// Visits a nested table of options named `name`.
    fn record_set(&mut self, name: &str, set: OptionSet);
}

/// Returns metadata for the options of a configuration struct.
pub trait OptionsMetadata {
    /// Calls `visit` once for every option of this struct.
    fn record(visit: &mut dyn Visit);

    /// The struct-level doc comment, rendered above the table in the settings reference.
    #[must_use]
    fn documentation() -> Option<&'static str> {
        None
    }

    /// The extracted metadata.
    #[must_use]
    fn metadata() -> OptionSet
    where
        Self: Sized + 'static,
    {
        OptionSet::of::<Self>()
    }
}

/// An option that is either a single [`OptionField`] or a nested [`OptionSet`].
pub enum OptionEntry {
    /// A single option.
    Field(OptionField),
    /// A nested table of options.
    Set(OptionSet),
}

/// A set of options, extracted by calling [`OptionsMetadata::record`].
#[derive(Copy, Clone)]
pub struct OptionSet {
    record: fn(&mut dyn Visit),
    doc: fn() -> Option<&'static str>,
}

impl OptionSet {
    #[must_use]
    pub fn of<T>() -> Self
    where
        T: OptionsMetadata + 'static,
    {
        Self {
            record: T::record,
            doc: T::documentation,
        }
    }

    /// Visits the options in this set by calling `visit` for each one.
    pub fn record(self, visit: &mut dyn Visit) {
        (self.record)(visit);
    }

    #[must_use]
    pub fn documentation(self) -> Option<&'static str> {
        (self.doc)()
    }

    /// Looks up an option by its dotted path, e.g. `lint.unsorted-tailwind-classes.prefix`.
    #[must_use]
    pub fn find(self, name: &str) -> Option<OptionEntry> {
        struct FindVisitor<'a> {
            entry: Option<OptionEntry>,
            rest: std::str::Split<'a, char>,
            needle: &'a str,
        }

        impl Visit for FindVisitor<'_> {
            fn record_field(&mut self, name: &str, field: OptionField) {
                if self.entry.is_none() && name == self.needle && self.rest.next().is_none() {
                    self.entry = Some(OptionEntry::Field(field));
                }
            }

            fn record_set(&mut self, name: &str, set: OptionSet) {
                if self.entry.is_none() && name == self.needle {
                    if let Some(next) = self.rest.next() {
                        self.needle = next;
                        set.record(self);
                    } else {
                        self.entry = Some(OptionEntry::Set(set));
                    }
                }
            }
        }

        let mut rest = name.split('.');
        let first = rest.next()?;
        let mut visitor = FindVisitor {
            entry: None,
            rest,
            needle: first,
        };
        self.record(&mut visitor);
        visitor.entry
    }
}

/// A single option, as declared by a `#[option(…)]` attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionField {
    /// The field's doc comment.
    pub doc: &'static str,
    /// The default value, rendered as TOML. Ex) `"false"`
    pub default: &'static str,
    /// Ex) `"bool"`
    pub value_type: &'static str,
    /// The TOML sub-table the example belongs to. Ex) `"per-file-ignores"`
    pub scope: Option<&'static str>,
    /// A TOML snippet setting this option.
    pub example: &'static str,
}

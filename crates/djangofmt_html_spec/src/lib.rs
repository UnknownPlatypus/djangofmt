//! HTML specification data for djangofmt linting.
//!
//! This crate provides compile-time static data for HTML elements and attributes,
//! sourced from [markuplint's html-spec](https://github.com/markuplint/markuplint).
//!
//! The data is fetched at build time and compiled into perfect hash maps for
//! constant-time lookups.

pub mod types;

pub use types::{AttributeSpec, AttributeValueType, ElementSpec};

#[allow(clippy::unreadable_literal)]
mod generated {
    use crate::types::{AttributeSpec, AttributeValueType, ElementSpec};
    include!(concat!(env!("OUT_DIR"), "/generated_specs.rs"));
}

pub use generated::{ELEMENTS, GLOBAL_ATTRS};

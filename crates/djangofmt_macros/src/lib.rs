//! Internal proc-macros for the djangofmt crates.

use proc_macro::TokenStream;
use syn::{DeriveInput, Error, parse_macro_input};

mod violation_metadata;

/// Derives `djangofmt_lint::ViolationMetadata` for a lint violation struct.
///
/// The derive captures the struct's `///` doc comment as the rule's explanation,
/// plus the source file and line of the struct definition.
#[proc_macro_derive(ViolationMetadata)]
pub fn derive_violation_metadata(item: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(item);
    violation_metadata::derive(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

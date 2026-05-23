//! Internal proc-macros for the djangofmt crates.

use proc_macro::TokenStream;
use syn::{DeriveInput, Error, ItemFn, parse_macro_input};

mod derive_message_formats;
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

/// Emits a sibling `message_formats()` for a `Violation::message` impl.
///
/// The returned slice contains every static format string the body could
/// produce; the docs generator uses it to populate the Message column of
/// the rules table.
///
/// Ruff equivalent: `ruff_macros::derive_message_formats`.
#[proc_macro_attribute]
pub fn derive_message_formats(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    derive_message_formats::derive(&func).into()
}

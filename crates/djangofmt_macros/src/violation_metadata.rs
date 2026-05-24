use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::meta::ParseNestedMeta;
use syn::{Attribute, DeriveInput, Error, Lit, LitStr, Meta};

pub fn derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let docs = collect_docs(&input.attrs);
    let Some(group) = collect_rule_group(&input.attrs)? else {
        return Err(Error::new_spanned(
            &input,
            "missing required `#[violation_metadata(stable_since = \"…\")]` \
             (or `preview_since` / `deprecated_since` / `removed_since`)",
        ));
    };
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = &input.generics.split_for_impl();

    // return `None` when the struct has no doc comment so the docs generator can skip
    // undocumented rules instead of writing an empty Markdown file.
    let explain_body = if docs.is_empty() {
        quote! { None }
    } else {
        quote! { Some(#docs) }
    };

    // Anchor `file!()` / `line!()` at the struct ident's span so the compiler resolves them to
    // the struct's source location instead of the `#[derive(ViolationMetadata)]` attribute line.
    let span = name.span();
    let file_expr = quote_spanned! { span => file!() };
    let line_expr = quote_spanned! { span => line!() };

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics crate::ViolationMetadata for #name #ty_generics #where_clause {
            fn explain() -> Option<&'static str> {
                #explain_body
            }

            fn group() -> crate::registry::RuleGroup {
                crate::registry::#group
            }

            fn file() -> &'static str {
                #file_expr
            }

            fn line() -> u32 {
                #line_expr
            }
        }
    })
}

/// Parse the `#[violation_metadata(stable_since = "…")]` helper attribute into a `RuleGroup::Variant` expression.
///
/// Errors if more than one lifecycle key is present so a typo like
/// `(stable_since = "…", preview_since = "…")` can't silently pick whichever key the parser visited last.
fn collect_rule_group(attrs: &[Attribute]) -> syn::Result<Option<TokenStream>> {
    let mut group = None;
    for attr in attrs {
        if !attr.path().is_ident("violation_metadata") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            let constructor: fn(&LitStr) -> TokenStream = if meta.path.is_ident("stable_since") {
                |lit| quote! { RuleGroup::Stable { since: #lit } }
            } else if meta.path.is_ident("preview_since") {
                |lit| quote! { RuleGroup::Preview { since: #lit } }
            } else if meta.path.is_ident("deprecated_since") {
                |lit| quote! { RuleGroup::Deprecated { since: #lit } }
            } else if meta.path.is_ident("removed_since") {
                |lit| quote! { RuleGroup::Removed { since: #lit } }
            } else {
                return Err(meta.error("unknown `violation_metadata` option"));
            };
            if group.is_some() {
                return Err(meta.error(
                    "duplicate lifecycle key; expected exactly one of \
                     `stable_since`, `preview_since`, `deprecated_since`, `removed_since`",
                ));
            }
            let lit = parse_version(&meta)?;
            group = Some(constructor(&lit));
            Ok(())
        })?;
    }
    Ok(group)
}

/// Accept a dotted numeric version (e.g. `"0.2.5"`) or the `NEXT_DJANGOFMT_VERSION` placeholder,
/// which release tooling rewrites at tag time. Reject everything else — in particular a leading
/// `v` prefix or stray letters that would later trip the docs URL builder.
fn parse_version(meta: &ParseNestedMeta) -> syn::Result<LitStr> {
    let lit: LitStr = meta.value()?.parse()?;
    let value = lit.value();
    let is_placeholder = value == "NEXT_DJANGOFMT_VERSION";
    let mut parts = value.split('.');
    let is_numeric_semver = match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(major), Some(minor), Some(patch), None) => [major, minor, patch]
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit())),
        _ => false,
    };
    if is_placeholder || is_numeric_semver {
        Ok(lit)
    } else {
        Err(Error::new_spanned(
            &lit,
            format!(
                "unrecognized version `{value}` (expected `MAJOR.MINOR.PATCH` or `NEXT_DJANGOFMT_VERSION`)"
            ),
        ))
    }
}

fn collect_docs(attrs: &[Attribute]) -> String {
    let mut out = String::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        let Some(lit) = doc_attr_lit(attr) else {
            continue;
        };
        let value = lit.value();
        let line = value.strip_prefix(' ').unwrap_or(&value);
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn doc_attr_lit(attr: &Attribute) -> Option<&LitStr> {
    let Meta::NameValue(nv) = &attr.meta else {
        return None;
    };
    if !nv.path.is_ident("doc") {
        return None;
    }
    let syn::Expr::Lit(syn::ExprLit {
        lit: Lit::Str(lit), ..
    }) = &nv.value
    else {
        return None;
    };
    Some(lit)
}

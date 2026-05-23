use proc_macro2::TokenStream;
use quote::quote;
use syn::meta::ParseNestedMeta;
use syn::{Attribute, DeriveInput, Error, Lit, LitStr, Meta};

pub fn derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let docs = collect_docs(&input.attrs)?;
    let Some(group) = collect_rule_group(&input.attrs)? else {
        return Err(Error::new_spanned(
            &input,
            "missing required `#[violation_metadata(stable_since = \"…\")]` \
             (or `preview_since` / `deprecated_since` / `removed_since`)",
        ));
    };
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = &input.generics.split_for_impl();

    // Mirror ruff: return `None` when the struct has no doc comment so the
    // docs generator can skip undocumented rules instead of writing an
    // empty Markdown file.
    let explain_body = if docs.is_empty() {
        quote! { None }
    } else {
        quote! { Some(#docs) }
    };

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
                file!()
            }

            fn line() -> u32 {
                line!()
            }
        }
    })
}

/// Parse the `#[violation_metadata(stable_since = "…")]` helper attribute
/// (one of `stable_since`, `preview_since`, `deprecated_since`,
/// `removed_since`) into a `RuleGroup::Variant { since: "…" }` expression.
///
/// Ruff equivalent: `ruff_macros::violation_metadata::get_rule_status`.
fn collect_rule_group(attrs: &[Attribute]) -> syn::Result<Option<TokenStream>> {
    let mut group = None;
    for attr in attrs {
        if !attr.path().is_ident("violation_metadata") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("stable_since") {
                let lit = parse_version(&meta)?;
                group = Some(quote! { RuleGroup::Stable { since: #lit } });
                Ok(())
            } else if meta.path.is_ident("preview_since") {
                let lit = parse_version(&meta)?;
                group = Some(quote! { RuleGroup::Preview { since: #lit } });
                Ok(())
            } else if meta.path.is_ident("deprecated_since") {
                let lit = parse_version(&meta)?;
                group = Some(quote! { RuleGroup::Deprecated { since: #lit } });
                Ok(())
            } else if meta.path.is_ident("removed_since") {
                let lit = parse_version(&meta)?;
                group = Some(quote! { RuleGroup::Removed { since: #lit } });
                Ok(())
            } else {
                Err(meta.error("unknown `violation_metadata` option"))
            }
        })?;
    }
    Ok(group)
}

fn parse_version(meta: &ParseNestedMeta) -> syn::Result<LitStr> {
    // Accept a semver string (e.g. "0.2.5") or the `NEXT_DJANGOFMT_VERSION`
    // placeholder, which a release tool may substitute. Loose validation —
    // we only check that the literal is non-empty and made of digits, dots,
    // or the placeholder so we catch obvious typos.
    let lit: LitStr = meta.value()?.parse()?;
    let value = lit.value();
    let is_placeholder = value == "NEXT_DJANGOFMT_VERSION";
    let is_semver = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c.is_ascii_alphabetic())
        && value.chars().any(|c| c.is_ascii_digit());
    if is_placeholder || is_semver {
        Ok(lit)
    } else {
        Err(Error::new_spanned(
            &lit,
            format!("unrecognized version `{value}`"),
        ))
    }
}

fn collect_docs(attrs: &[Attribute]) -> syn::Result<String> {
    let mut out = String::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        let Some(lit) = doc_attr_lit(attr) else {
            return Err(Error::new_spanned(attr, "unimplemented doc comment style"));
        };
        let value = lit.value();
        let line = value.strip_prefix(' ').unwrap_or(&value);
        out.push_str(line);
        out.push('\n');
    }
    Ok(out)
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

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, DeriveInput, Error, Lit, LitStr, Meta};

pub fn derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let docs = collect_docs(&input.attrs)?;
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = &input.generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics crate::ViolationMetadata for #name #ty_generics #where_clause {
            fn explain() -> &'static str {
                #docs
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

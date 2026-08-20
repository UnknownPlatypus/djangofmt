use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    Attribute, Data, DataStruct, DeriveInput, Error, Field, Fields, GenericArgument, LitStr,
    PathArguments, Type,
};

use crate::violation_metadata::collect_docs;

pub fn derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let Data::Struct(DataStruct {
        fields: Fields::Named(fields),
        ..
    }) = input.data
    else {
        return Err(Error::new_spanned(
            &input.ident,
            "`OptionsMetadata` can only be derived for structs with named fields",
        ));
    };

    let mut records = Vec::new();
    for field in &fields.named {
        if let Some(attr) = field
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("option"))
        {
            records.push(record_field(field, attr)?);
        } else if field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("option_group"))
        {
            records.push(record_set(field)?);
        }
    }

    let docs = collect_docs(&input.attrs);
    let docs = docs.trim();
    let documentation = if docs.is_empty() {
        quote!()
    } else {
        quote! {
            fn documentation() -> Option<&'static str> {
                Some(#docs)
            }
        }
    };

    let name = input.ident;
    Ok(quote! {
        #[automatically_derived]
        impl crate::options_metadata::OptionsMetadata for #name {
            fn record(visit: &mut dyn crate::options_metadata::Visit) {
                #(#records)*
            }

            #documentation
        }
    })
}

/// Build a `record_field` call from a `#[option(default = "…", value_type = "…", example = "…")]`
/// attribute and the field's doc comment.
fn record_field(field: &Field, attr: &Attribute) -> syn::Result<TokenStream> {
    let ident = field.ident.as_ref().expect("named field");
    let doc = collect_docs(&field.attrs);
    let doc = doc.trim();
    if doc.is_empty() {
        return Err(Error::new_spanned(
            field,
            "missing documentation for option",
        ));
    }

    let mut default = None;
    let mut value_type = None;
    let mut example = None;
    let mut scope = None;
    attr.parse_nested_meta(|meta| {
        let target = if meta.path.is_ident("default") {
            &mut default
        } else if meta.path.is_ident("value_type") {
            &mut value_type
        } else if meta.path.is_ident("example") {
            &mut example
        } else if meta.path.is_ident("scope") {
            &mut scope
        } else {
            return Err(meta.error("unknown `option` key"));
        };
        *target = Some(meta.value()?.parse::<LitStr>()?.value());
        Ok(())
    })?;

    let missing = |key: &str| Error::new_spanned(attr, format!("missing `{key}` in `#[option]`"));
    let default = default.ok_or_else(|| missing("default"))?;
    let value_type = value_type.ok_or_else(|| missing("value_type"))?;
    let example = dedent(&example.ok_or_else(|| missing("example"))?);
    let scope = scope.map_or_else(|| quote!(None), |scope| quote!(Some(#scope)));
    let name = LitStr::new(&ident.to_string().replace('_', "-"), ident.span());

    Ok(quote_spanned! { ident.span() =>
        visit.record_field(#name, crate::options_metadata::OptionField {
            doc: #doc,
            default: #default,
            value_type: #value_type,
            scope: #scope,
            example: #example,
        });
    })
}

/// Build a `record_set` call for an `#[option_group]` field, whose type must be `Option<T>` with
/// `T` deriving `OptionsMetadata`.
fn record_set(field: &Field) -> syn::Result<TokenStream> {
    let ident = field.ident.as_ref().expect("named field");
    let inner = option_inner_type(&field.ty).ok_or_else(|| {
        Error::new_spanned(&field.ty, "expected `Option<_>` for `#[option_group]`")
    })?;
    let name = LitStr::new(&ident.to_string().replace('_', "-"), ident.span());

    Ok(quote_spanned! { ident.span() =>
        visit.record_set(#name, crate::options_metadata::OptionSet::of::<#inner>());
    })
}

fn option_inner_type(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else { return None };
    let segment = path.path.segments.first()?;
    if segment.ident != "Option" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    match args.args.first()? {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    }
}

/// Strip the common leading indentation of a multi-line `example`, so it can be written inline
/// with the surrounding code.
fn dedent(text: &str) -> String {
    let indent = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    text.lines()
        .map(|line| line.get(indent..).unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
        .trim_matches('\n')
        .to_string()
}

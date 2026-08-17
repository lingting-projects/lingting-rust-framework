use quote::ToTokens;
use syn::{Attribute, Field, Token, parse_quote, punctuated::Punctuated, spanned::Spanned};

pub fn ensure_item_serde_as(attrs: &mut Vec<Attribute>) {
    let position = attrs.iter().position(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "serde_as")
    });
    if let Some(position) = position {
        let attribute = attrs.remove(position);
        attrs.insert(0, attribute);
    } else {
        attrs.insert(0, parse_quote!(#[serde_with::serde_as]));
    }
}

pub fn ensure_item_serde_rename_all(attrs: &mut Vec<Attribute>, value: &str) {
    if has_attribute_key(attrs, "serde", "rename_all") {
        return;
    }
    let value = syn::LitStr::new(value, proc_macro2::Span::call_site());
    attrs.push(parse_quote!(#[serde(rename_all = #value)]));
}

pub fn ensure_item_strum_serialize_all(attrs: &mut Vec<Attribute>, value: &str) {
    if has_attribute_key(attrs, "strum", "serialize_all") {
        return;
    }
    let value = syn::LitStr::new(value, proc_macro2::Span::call_site());
    attrs.push(parse_quote!(#[strum(serialize_all = #value)]));
}

pub fn ensure_item_strum_ascii_case_insensitive(attrs: &mut Vec<Attribute>) {
    if has_attribute_key(attrs, "strum", "ascii_case_insensitive") {
        return;
    }
    attrs.push(parse_quote!(#[strum(ascii_case_insensitive)]));
}

pub fn ensure_serde_default(attrs: &mut Vec<Attribute>) {
    if has_attribute_key(attrs, "serde", "default") {
        return;
    }
    attrs.push(parse_quote!(#[serde(default)]));
}

pub fn ensure_serde_adapter(attrs: &mut Vec<Attribute>, adapter: &str) {
    if has_attribute_key(attrs, "serde_as", "as") {
        return;
    }
    let value = syn::LitStr::new(adapter, proc_macro2::Span::call_site());
    attrs.push(parse_quote!(#[serde_as(as = #value)]));
}

pub fn ensure_specta_type(attrs: &mut Vec<Attribute>, ty: proc_macro2::TokenStream) {
    if has_attribute_key(attrs, "specta", "type") {
        return;
    }
    attrs.push(parse_quote!(#[specta(type = #ty)]));
}

pub fn attribute_metadata(
    attrs: &[Attribute],
    fields: Option<&Punctuated<Field, Token![,]>>,
) -> Vec<syn::LitStr> {
    let mut values = attrs
        .iter()
        .filter(|attr| !attr.path().is_ident("derive"))
        .map(|attr| metadata_literal("item", attr))
        .collect::<Vec<_>>();

    if let Some(fields) = fields {
        for field in fields {
            let Some(ident) = &field.ident else {
                continue;
            };
            let target = format!("field:{ident}");
            values.extend(
                field
                    .attrs
                    .iter()
                    .map(|attr| metadata_literal(&target, attr)),
            );
        }
    }
    values
}

fn has_attribute_key(attrs: &[Attribute], name: &str, key: &str) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == name)
            && attr
            .meta
            .to_token_stream()
            .to_string()
            .split(|character: char| !character.is_alphanumeric() && character != '_')
            .any(|part| part == key)
    })
}

fn metadata_literal(target: &str, attr: &Attribute) -> syn::LitStr {
    let value = format!(
        "{target}:{}",
        attr.to_token_stream().to_string().replace(' ', "")
    );
    syn::LitStr::new(&value, attr.span())
}

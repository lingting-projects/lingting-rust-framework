use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Attribute, Path, Token, parse_quote, punctuated::Punctuated, spanned::Spanned};

#[derive(Clone, Copy)]
pub struct DeriveOption {
    name: &'static str,
    enabled: bool,
}

impl DeriveOption {
    pub const fn new(name: &'static str, enabled: bool) -> Self {
        Self { name, enabled }
    }
}

pub fn ensure_derives(attrs: &mut Vec<Attribute>, options: &[DeriveOption]) {
    let mut paths = Vec::new();
    let mut remaining = Vec::new();

    for attr in attrs.drain(..) {
        if attr.path().is_ident("derive") {
            if let Ok(items) = attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
            {
                paths.extend(items);
            } else {
                remaining.push(attr);
            }
        } else {
            remaining.push(attr);
        }
    }

    for option in options {
        if option.enabled {
            push_missing(&mut paths, path_for(option.name));
        }
    }

    let tokens: Vec<TokenStream> = paths.iter().map(ToTokens::to_token_stream).collect();
    remaining.insert(0, parse_quote!(#[derive(#(#tokens),*)]));
    *attrs = remaining;
}

pub fn derive_metadata(attrs: &[Attribute]) -> Vec<syn::LitStr> {
    let mut values = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        let Ok(paths) = attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
        else {
            continue;
        };
        for path in paths {
            let value = canonical_derive_name(&path);
            if !values
                .iter()
                .any(|item: &syn::LitStr| item.value() == value)
            {
                values.push(syn::LitStr::new(&value, path.span()));
            }
        }
    }
    values
}

fn push_missing(paths: &mut Vec<Path>, path: Path) {
    let name = path_last_segment(&path);
    if paths
        .iter()
        .any(|existing| path_last_segment(existing) == name)
    {
        return;
    }
    paths.push(path);
}

fn path_for(name: &str) -> Path {
    match name {
        "Default" => parse_quote!(Default),
        "Debug" => parse_quote!(Debug),
        "Clone" => parse_quote!(Clone),
        "Copy" => parse_quote!(Copy),
        "PartialEq" => parse_quote!(PartialEq),
        "Eq" => parse_quote!(Eq),
        "Serialize" => parse_quote!(serde::Serialize),
        "Deserialize" => parse_quote!(serde::Deserialize),
        "Type" => parse_quote!(specta::Type),
        "EnumIter" => parse_quote!(strum_macros::EnumIter),
        "EnumString" => parse_quote!(strum_macros::EnumString),
        "Display" => parse_quote!(strum_macros::Display),
        _ => unreachable!("不支持的派生项"),
    }
}

fn canonical_derive_name(path: &Path) -> String {
    match path_last_segment(path).as_str() {
        "Default" => "std::default::Default".to_string(),
        "Debug" => "std::fmt::Debug".to_string(),
        "Clone" => "std::clone::Clone".to_string(),
        "Copy" => "std::marker::Copy".to_string(),
        "PartialEq" => "std::cmp::PartialEq".to_string(),
        "Eq" => "std::cmp::Eq".to_string(),
        _ => path.to_token_stream().to_string().replace(' ', ""),
    }
}

fn path_last_segment(path: &Path) -> String {
    path.segments
        .last()
        .map(|segment| segment.ident.to_string())
        .unwrap_or_default()
}

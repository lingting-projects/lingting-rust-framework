use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Attribute, Fields, ItemEnum, ItemStruct, Path, Token, Visibility, parse_quote,
    punctuated::Punctuated,
};

/// 定义快照中保留的派生项；按路径最后一段比较。
const RETAINED_DERIVES: &[&str] = &[
    "Clone",
    "Copy",
    "Debug",
    "PartialEq",
    "Eq",
    "Default",
    "Serialize",
    "Deserialize",
];

/// 生成结构体的定义快照：可见性提升为 `pub`，属性按白名单清洗。
pub fn struct_definition(item: &ItemStruct) -> TokenStream {
    let mut item = item.clone();
    item.vis = public_visibility();
    item.attrs = definition_attributes(&item.attrs);
    clean_fields(&mut item.fields);
    item.to_token_stream()
}

/// 生成枚举的定义快照：可见性提升为 `pub`，属性按白名单清洗。
pub fn enum_definition(item: &ItemEnum) -> TokenStream {
    let mut item = item.clone();
    item.vis = public_visibility();
    item.attrs = definition_attributes(&item.attrs);
    for variant in &mut item.variants {
        variant.attrs = definition_attributes(&variant.attrs);
        clean_fields(&mut variant.fields);
    }
    item.to_token_stream()
}

fn public_visibility() -> Visibility {
    parse_quote!(pub)
}

fn clean_fields(fields: &mut Fields) {
    for field in fields.iter_mut() {
        field.vis = public_visibility();
        field.attrs = definition_attributes(&field.attrs);
    }
}

fn definition_attributes(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs.iter().filter_map(definition_attribute).collect()
}

fn definition_attribute(attr: &Attribute) -> Option<Attribute> {
    if attr.path().is_ident("derive") {
        return derive_attribute(attr);
    }
    let name = attr.path().segments.last()?.ident.to_string();
    matches!(name.as_str(), "serde" | "serde_as" | "doc").then(|| attr.clone())
}

fn derive_attribute(attr: &Attribute) -> Option<Attribute> {
    let paths = attr
        .parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
        .ok()?;
    let paths = paths
        .into_iter()
        .filter(is_retained_derive)
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return None;
    }
    Some(parse_quote!(#[derive(#(#paths),*)]))
}

fn is_retained_derive(path: &Path) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| RETAINED_DERIVES.contains(&segment.ident.to_string().as_str()))
}

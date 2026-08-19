mod attributes;
mod collection;
mod derives;
mod enum_impl;
mod fields;
mod options;
mod type_transform;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Error, Fields, GenericParam, ItemEnum, ItemImpl, ItemStruct, parse2};

use crate::attributes::{
    attribute_metadata, ensure_item_serde_as, ensure_item_serde_rename_all,
    ensure_item_strum_ascii_case_insensitive, ensure_item_strum_serialize_all,
};
use crate::derives::{derive_metadata, ensure_derives};
use crate::fields::transform_fields;
use crate::options::{AutoEnumOptions, AutoTypeOptions};

#[proc_macro_attribute]
pub fn auto_type(args: TokenStream, input: TokenStream) -> TokenStream {
    expand_auto_type(args.into(), input.into()).into()
}

#[proc_macro_attribute]
pub fn auto_enum(args: TokenStream, input: TokenStream) -> TokenStream {
    expand_auto_enum(args.into(), input.into()).into()
}

#[proc_macro_attribute]
pub fn auto_enum_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    expand_auto_enum_impl(args.into(), input.into()).into()
}

#[proc_macro_attribute]
pub fn auto_enum_field(_: TokenStream, _: TokenStream) -> TokenStream {
    Error::new(
        proc_macro2::Span::call_site(),
        "auto_enum_field 仅能用于 #[auto_enum_impl] 标记的固有 impl 方法",
    )
    .to_compile_error()
    .into()
}

fn expand_auto_type(args: TokenStream2, input: TokenStream2) -> TokenStream2 {
    let options = match AutoTypeOptions::parse(args) {
        Ok(options) => options,
        Err(error) => return error.to_compile_error(),
    };
    let mut item = match parse2::<ItemStruct>(input) {
        Ok(item) => item,
        Err(error) => return error.to_compile_error(),
    };

    if let Err(error) = ensure_type_generics(&item.generics) {
        return error.to_compile_error();
    }
    let field_summary = match &mut item.fields {
        Fields::Named(fields) => transform_fields(&mut fields.named, options.serde, options.specta),
        _ => {
            return Error::new_spanned(&item.fields, "auto_type 仅支持具名字段结构体")
                .to_compile_error();
        }
    };
    let derives = options.derive_options();
    ensure_derives(&mut item.attrs, &derives);
    if options.serde && field_summary.requires_serde_as {
        ensure_item_serde_as(&mut item.attrs);
    }
    if options.serde {
        ensure_item_serde_rename_all(&mut item.attrs, "camelCase");
    }
    let ident = item.ident.clone();
    let attributes = match &item.fields {
        Fields::Named(fields) => attribute_metadata(&item.attrs, Some(&fields.named)),
        _ => unreachable!("字段类型已在前面校验"),
    };

    let collection = collection::expand(
        &ident,
        &item.generics,
        quote!(::framework_proc_core::TypeKind::Struct),
        derive_metadata(&item.attrs),
        attributes,
        options.specta,
    );
    quote!(#item #collection)
}

fn expand_auto_enum_impl(args: TokenStream2, input: TokenStream2) -> TokenStream2 {
    if !args.is_empty() {
        return Error::new_spanned(args, "auto_enum_impl 不支持参数").to_compile_error();
    }
    let item = match parse2::<ItemImpl>(input) {
        Ok(item) => item,
        Err(error) => return error.to_compile_error(),
    };
    match enum_impl::expand(item) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error(),
    }
}

fn ensure_type_generics(generics: &syn::Generics) -> syn::Result<()> {
    for parameter in &generics.params {
        if !matches!(parameter, GenericParam::Type(_)) {
            return Err(Error::new_spanned(
                parameter,
                "auto_type 的 TypeScript 元数据收集仅支持类型泛型",
            ));
        }
    }
    Ok(())
}

fn expand_auto_enum(args: TokenStream2, input: TokenStream2) -> TokenStream2 {
    let options = match AutoEnumOptions::parse(args) {
        Ok(options) => options,
        Err(error) => return error.to_compile_error(),
    };
    let mut item = match parse2::<ItemEnum>(input) {
        Ok(item) => item,
        Err(error) => return error.to_compile_error(),
    };

    if !item.generics.params.is_empty() {
        return Error::new_spanned(&item.generics, "auto_enum 不支持泛型枚举").to_compile_error();
    }

    let derives = options.derive_options();
    ensure_derives(&mut item.attrs, &derives);
    if options.strum {
        ensure_item_strum_serialize_all(&mut item.attrs, "UPPERCASE");
        ensure_item_strum_ascii_case_insensitive(&mut item.attrs);
    }
    if options.serde {
        ensure_item_serde_rename_all(&mut item.attrs, "UPPERCASE");
    }
    let ident = item.ident.clone();

    let collection = collection::expand(
        &ident,
        &item.generics,
        quote!(::framework_proc_core::TypeKind::Enum),
        derive_metadata(&item.attrs),
        attribute_metadata(&item.attrs, None),
        options.specta,
    );
    quote!(#item #collection)
}

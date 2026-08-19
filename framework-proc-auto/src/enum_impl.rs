use proc_macro2::TokenStream;
#[cfg(feature = "collect")]
use quote::format_ident;
use quote::quote;
use syn::{Attribute, Error, ImplItem, ItemImpl, punctuated::Punctuated};
#[cfg(feature = "collect")]
use syn::{FnArg, ImplItemFn, LitStr, ReturnType, Type};

#[cfg(feature = "collect")]
use crate::type_transform::{TypeTransform, TypeWrapper};

pub fn expand(mut item: ItemImpl) -> syn::Result<TokenStream> {
    if item.trait_.is_some() {
        return Err(Error::new_spanned(
            &item.self_ty,
            "auto_enum_impl 仅支持枚举的固有 impl",
        ));
    }
    if !item.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &item.generics,
            "auto_enum_impl 不支持泛型 impl",
        ));
    }

    #[cfg(feature = "collect")]
    let enum_ident = enum_ident(&item.self_ty)?;
    #[cfg(feature = "collect")]
    let mut fields = Vec::new();
    for impl_item in &mut item.items {
        let ImplItem::Fn(method) = impl_item else {
            continue;
        };
        let marker = take_field_marker(&mut method.attrs)?;
        #[cfg(feature = "collect")]
        if method.sig.ident == "label" && marker.is_none() {
            let value = enum_field_value(method)?;
            fields.push((LitStr::new("label", method.sig.ident.span()), value));
        } else if let Some(name) = marker {
            let value = enum_field_value(method)?;
            let name = name.unwrap_or_else(|| camel_case(&method.sig.ident.to_string()));
            fields.push((LitStr::new(&name, method.sig.ident.span()), value));
        }
        #[cfg(not(feature = "collect"))]
        let _ = marker;
    }

    #[cfg(feature = "collect")]
    {
        let mut names = std::collections::HashSet::new();
        for (name, _) in &fields {
            if !names.insert(name.value()) {
                return Err(Error::new_spanned(name, "auto_enum_field 字段名不能重复"));
            }
        }

        let type_name_fn = format_ident!("__proc_auto_enum_name_{enum_ident}");
        let values_fn = format_ident!("__proc_auto_enum_values_{enum_ident}");
        let field_names = fields.iter().map(|(name, _)| name);
        let values = fields.iter().map(|(name, value)| quote!((#name, #value)));
        let self_ty = &item.self_ty;

        Ok(quote! {
        #item

        #[doc(hidden)]
        fn #type_name_fn() -> &'static str {
            ::std::any::type_name::<#self_ty>()
        }

        #[doc(hidden)]
        fn #values_fn() -> ::std::result::Result<
            ::std::vec::Vec<::framework_proc_core::EnumValue>,
            ::framework_proc_core::serde_json::Error,
        > {
            <#self_ty as ::strum::IntoEnumIterator>::iter()
                .map(|value| {
                    Ok(::framework_proc_core::EnumValue {
                        value: value.to_string(),
                        fields: ::std::vec![#(#values),*],
                    })
                })
                .collect()
        }

        ::framework_proc_core::push_enum_metadata! {
            ::framework_proc_core::EnumMetadata {
                type_name: #type_name_fn,
                fields: &[#(#field_names),*],
                values: #values_fn,
            }
        }
        })
    }

    #[cfg(not(feature = "collect"))]
    Ok(quote!(#item))
}

#[cfg(feature = "collect")]
fn enum_ident(ty: &Type) -> syn::Result<&syn::Ident> {
    let Type::Path(path) = ty else {
        return Err(Error::new_spanned(
            ty,
            "auto_enum_impl 的 Self 类型必须是枚举路径",
        ));
    };
    let Some(segment) = path.path.segments.last() else {
        return Err(Error::new_spanned(
            ty,
            "auto_enum_impl 的 Self 类型不能为空",
        ));
    };
    if !segment.arguments.is_empty() {
        return Err(Error::new_spanned(
            segment,
            "auto_enum_impl 不支持带泛型参数的 Self 类型",
        ));
    }
    Ok(&segment.ident)
}

fn take_field_marker(attrs: &mut Vec<Attribute>) -> syn::Result<Option<Option<String>>> {
    let mut field = None;
    attrs.retain(|attr| {
        if !attr.path().is_ident("auto_enum_field") {
            return true;
        }
        if field.is_some() {
            field = Some(Err(Error::new_spanned(attr, "auto_enum_field 不能重复")));
            return false;
        }
        field = Some(parse_field_marker(attr));
        false
    });
    field.transpose()
}

fn parse_field_marker(attr: &Attribute) -> syn::Result<Option<String>> {
    if matches!(&attr.meta, syn::Meta::Path(_)) {
        return Ok(None);
    }
    let values = attr.parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)?;
    let mut field = None;
    for value in values {
        let syn::Meta::NameValue(value) = value else {
            return Err(Error::new_spanned(
                value,
                "auto_enum_field 参数必须使用 field = \"...\"",
            ));
        };
        if !value.path.is_ident("field") || field.is_some() {
            return Err(Error::new_spanned(
                value,
                "auto_enum_field 仅支持一个 field 参数",
            ));
        }
        let syn::Expr::Lit(value) = value.value else {
            return Err(Error::new_spanned(
                value,
                "auto_enum_field 的 field 必须是字符串",
            ));
        };
        let syn::Lit::Str(value) = value.lit else {
            return Err(Error::new_spanned(
                value,
                "auto_enum_field 的 field 必须是字符串",
            ));
        };
        if value.value().is_empty() {
            return Err(Error::new_spanned(
                value,
                "auto_enum_field 的 field 不能为空",
            ));
        }
        field = Some(value.value());
    }
    Ok(field)
}

#[cfg(feature = "collect")]
fn validate_field_method(method: &ImplItemFn) -> syn::Result<()> {
    if method.sig.asyncness.is_some() || !method.sig.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &method.sig,
            "枚举导出字段方法不能是 async 或泛型方法",
        ));
    }
    if !matches!(method.sig.inputs.first(), Some(FnArg::Receiver(_))) {
        return Err(Error::new_spanned(
            &method.sig,
            "枚举导出字段方法必须接收 self",
        ));
    }
    if method.sig.inputs.len() != 1 {
        return Err(Error::new_spanned(
            &method.sig,
            "枚举导出字段方法不能接收额外参数",
        ));
    }
    if matches!(method.sig.output, ReturnType::Default) {
        return Err(Error::new_spanned(
            &method.sig,
            "枚举导出字段方法必须返回可转换为字符串的值",
        ));
    }
    Ok(())
}

#[cfg(feature = "collect")]
fn enum_field_value(method: &ImplItemFn) -> syn::Result<TokenStream> {
    validate_field_method(method)?;
    let ReturnType::Type(_, ty) = &method.sig.output else {
        return Err(Error::new_spanned(
            &method.sig,
            "枚举导出字段方法必须返回可转换为字符串的值",
        ));
    };
    let method = &method.sig.ident;
    let transform = TypeTransform::from_type(ty);
    if !transform.requires_string {
        return Ok(quote!(::framework_proc_core::enum_field_value(value.#method())?));
    }
    Ok(match transform.wrapper {
        TypeWrapper::Plain => quote!(::framework_proc_core::serde_json::Value::String(
            value.#method().to_string(),
        )),
        TypeWrapper::Option => quote!(match value.#method() {
            Some(value) => ::framework_proc_core::serde_json::Value::String(value.to_string()),
            None => ::framework_proc_core::serde_json::Value::Null,
        }),
        TypeWrapper::Vec => quote!(::framework_proc_core::serde_json::Value::Array(
            value.#method()
                .into_iter()
                .map(|value| ::framework_proc_core::serde_json::Value::String(value.to_string()))
                .collect(),
        )),
        TypeWrapper::OptionVec => quote!(match value.#method() {
            Some(values) => ::framework_proc_core::serde_json::Value::Array(
                values
                    .into_iter()
                    .map(|value| ::framework_proc_core::serde_json::Value::String(value.to_string()))
                    .collect(),
            ),
            None => ::framework_proc_core::serde_json::Value::Null,
        }),
    })
}

#[cfg(feature = "collect")]
fn camel_case(name: &str) -> String {
    let mut value = String::new();
    let mut uppercase_next = false;
    for (index, character) in name.chars().enumerate() {
        if character == '_' || character == '-' {
            uppercase_next = !value.is_empty();
            continue;
        }
        if index == 0 {
            value.extend(character.to_lowercase());
        } else if uppercase_next {
            value.extend(character.to_uppercase());
            uppercase_next = false;
        } else {
            value.push(character);
        }
    }
    value
}

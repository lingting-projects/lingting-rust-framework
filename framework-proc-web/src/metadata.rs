use crate::common::{generic_type, is_type};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Error, FnArg, ItemFn, Pat, ReturnType, Type};

/// 依据路由函数签名生成 `ApiMetadata` 注册代码。
pub(crate) fn expand_api_metadata(
    function: &ItemFn,
    method: &str,
    path: &syn::LitStr,
) -> syn::Result<TokenStream2> {
    let parameters = api_parameters(function)?;
    let return_type = api_return_type(&function.sig.output)?;
    let ident = &function.sig.ident;
    let names = parameters.iter().map(|parameter| &parameter.name);
    let ts_type_names = parameters.iter().map(|parameter| &parameter.ts_type_name);
    let rust_type_names = parameters.iter().map(|parameter| &parameter.rust_type_name);
    let kinds = parameters.iter().map(|parameter| &parameter.kind);
    Ok(quote! {
        ::framework_web::framework_proc_core::push_api_metadata! {
            ::framework_web::framework_proc_core::ApiMetadata {
                name: stringify!(#ident),
                namespace: module_path!(),
                method: #method,
                path: #path,
                parameters: &[
                    #(::framework_web::framework_proc_core::ApiParameterMetadata::new(
                        #names,
                        #kinds,
                        #ts_type_names,
                        #rust_type_names,
                    )),*
                ],
                return_type: #return_type,
            }
        }
    })
}

struct ApiParameter {
    name: syn::LitStr,
    ts_type_name: syn::LitStr,
    rust_type_name: syn::LitStr,
    kind: TokenStream2,
}

fn api_parameters(item: &ItemFn) -> syn::Result<Vec<ApiParameter>> {
    let mut parameters = Vec::new();
    for argument in &item.sig.inputs {
        let FnArg::Typed(argument) = argument else {
            return Err(Error::new_spanned(argument, "web_api 不支持 self 参数"));
        };
        let (kind, inner) = if let Some(inner) = generic_type(&argument.ty, "Json") {
            (
                quote!(::framework_web::framework_proc_core::ApiParameterKind::Body),
                inner,
            )
        } else if let Some(inner) = generic_type(&argument.ty, "Query") {
            (
                quote!(::framework_web::framework_proc_core::ApiParameterKind::Query),
                inner,
            )
        } else if is_type(&argument.ty, "PaginationParams") {
            (
                quote!(::framework_web::framework_proc_core::ApiParameterKind::Body),
                argument.ty.as_ref(),
            )
        } else {
            return Err(Error::new_spanned(
                &argument.ty,
                "生成 API 元数据时，参数仅支持 Json<T>、Query<T> 或 PaginationParams",
            ));
        };
        let name = pattern_name(&argument.pat)?;
        let ts_type_name = ts_type_name(inner)?;
        let rust_type_name = rust_type_name(inner)?;
        parameters.push(ApiParameter {
            name: syn::LitStr::new(&name, argument.pat.span()),
            ts_type_name: syn::LitStr::new(&ts_type_name, inner.span()),
            rust_type_name: syn::LitStr::new(&rust_type_name, inner.span()),
            kind,
        });
    }
    Ok(parameters)
}

fn pattern_name(pattern: &Pat) -> syn::Result<String> {
    match pattern {
        Pat::Ident(value) => Ok(value.ident.to_string()),
        Pat::TupleStruct(value) if value.elems.len() == 1 => pattern_name(&value.elems[0]),
        Pat::Type(value) => pattern_name(&value.pat),
        _ => Err(Error::new_spanned(
            pattern,
            "web_api 参数必须包含唯一绑定名",
        )),
    }
}

fn api_return_type(output: &ReturnType) -> syn::Result<TokenStream2> {
    let ReturnType::Type(_, ty) = output else {
        return Ok(quote!(
            ::framework_web::framework_proc_core::ApiReturnType::void()
        ));
    };
    let mut ty = ty.as_ref();
    while let Some(inner) = result_or_r_type(ty) {
        ty = inner;
    }
    if let Some(inner) = generic_type(ty, "WebResponseOf") {
        let ts_type_name = syn::LitStr::new(&ts_type_name(inner)?, inner.span());
        let rust_type_name = syn::LitStr::new(&rust_type_name(inner)?, inner.span());
        return Ok(quote!(
            ::framework_web::framework_proc_core::ApiReturnType::ty(#ts_type_name, #rust_type_name)
        ));
    }
    if is_type(ty, "WebResponse") {
        return Ok(quote!(
            ::framework_web::framework_proc_core::ApiReturnType::blob()
        ));
    }
    if is_unit_type(ty) {
        return Ok(quote!(
            ::framework_web::framework_proc_core::ApiReturnType::void()
        ));
    }
    let ts_type_name = syn::LitStr::new(&ts_type_name(ty)?, ty.span());
    let rust_type_name = syn::LitStr::new(&rust_type_name(ty)?, ty.span());
    Ok(quote!(
        ::framework_web::framework_proc_core::ApiReturnType::ty(#ts_type_name, #rust_type_name)
    ))
}

fn result_or_r_type(ty: &Type) -> Option<&Type> {
    generic_type(ty, "Result").or_else(|| generic_type(ty, "R"))
}

fn ts_type_name(ty: &Type) -> syn::Result<String> {
    let Type::Path(path) = ty else {
        return Err(Error::new_spanned(ty, "不支持该 TypeScript 类型"));
    };
    let Some(segment) = path.path.segments.last() else {
        return Err(Error::new_spanned(ty, "类型路径不能为空"));
    };
    let ident = segment.ident.to_string();
    let arguments = generic_arguments(segment, ts_type_name)?;
    if arguments.is_empty()
        && let Some(primitive) = primitive_type_name(&ident)
    {
        return Ok(primitive.to_string());
    }
    match ident.as_str() {
        "Option" => {
            let [inner] = arguments.as_slice() else {
                return Err(Error::new_spanned(ty, "Option 必须包含唯一泛型参数"));
            };
            Ok(format!("{inner} | null"))
        }
        "Vec" => {
            let [inner] = arguments.as_slice() else {
                return Err(Error::new_spanned(ty, "Vec 必须包含唯一泛型参数"));
            };
            Ok(format!("{inner}[]"))
        }
        "HashMap" | "BTreeMap" => {
            let [key, value] = arguments.as_slice() else {
                return Err(Error::new_spanned(ty, "Map 必须包含键和值两个泛型参数"));
            };
            Ok(format!("Record<{key}, {value}>"))
        }
        _ if arguments.is_empty() => Ok(ident),
        _ => Ok(format!("{ident}<{}>", arguments.join(", "))),
    }
}

/// Rust 原始类型对应的 TypeScript 类型名；不是原始类型时返回 `None`。
///
/// `i64` / `u64` / `i128` / `u128` 按项目约定序列化为字符串，
/// 与 `auto_type` 对同名字段的 `DisplayFromStr` 处理保持一致。
fn primitive_type_name(name: &str) -> Option<&'static str> {
    match name {
        "String" | "str" | "char" => Some("string"),
        "bool" => Some("boolean"),
        "i8" | "i16" | "i32" | "u8" | "u16" | "u32" | "isize" | "usize" | "f32" | "f64" => {
            Some("number")
        }
        "i64" | "u64" | "i128" | "u128" => Some("string"),
        _ => None,
    }
}

/// Rust 类型名本身，仅把 `str` / `char` 归一为 `String`。
fn rust_type_name(ty: &Type) -> syn::Result<String> {
    let Type::Path(path) = ty else {
        return Err(Error::new_spanned(ty, "不支持该 Rust 类型"));
    };
    let Some(segment) = path.path.segments.last() else {
        return Err(Error::new_spanned(ty, "类型路径不能为空"));
    };
    let ident = segment.ident.to_string();
    let arguments = generic_arguments(segment, rust_type_name)?;
    match ident.as_str() {
        "str" | "char" if arguments.is_empty() => return Ok("String".to_string()),
        "Option" => {
            let [inner] = arguments.as_slice() else {
                return Err(Error::new_spanned(ty, "Option 必须包含唯一泛型参数"));
            };
            return Ok(format!("Option<{inner}>"));
        }
        "Vec" => {
            let [inner] = arguments.as_slice() else {
                return Err(Error::new_spanned(ty, "Vec 必须包含唯一泛型参数"));
            };
            return Ok(format!("Vec<{inner}>"));
        }
        "HashMap" | "BTreeMap" => {
            let [key, value] = arguments.as_slice() else {
                return Err(Error::new_spanned(ty, "Map 必须包含键和值两个泛型参数"));
            };
            return Ok(format!("{ident}<{key}, {value}>"));
        }
        _ => {}
    }
    if arguments.is_empty() {
        return Ok(ident);
    }
    Ok(format!("{ident}<{}>", arguments.join(", ")))
}

fn generic_arguments(
    segment: &syn::PathSegment,
    convert: fn(&Type) -> syn::Result<String>,
) -> syn::Result<Vec<String>> {
    let arguments = match &segment.arguments {
        syn::PathArguments::None => return Ok(Vec::new()),
        syn::PathArguments::AngleBracketed(arguments) => arguments,
        syn::PathArguments::Parenthesized(arguments) => {
            return Err(Error::new_spanned(arguments, "不支持该 TypeScript 类型"));
        }
    };
    arguments
        .args
        .iter()
        .map(|argument| match argument {
            syn::GenericArgument::Type(ty) => convert(ty),
            argument => Err(Error::new_spanned(argument, "不支持该 TypeScript 类型")),
        })
        .collect()
}

fn is_unit_type(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(tuple) if tuple.elems.is_empty())
}

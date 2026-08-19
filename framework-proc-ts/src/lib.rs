use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::Parser;
use syn::spanned::Spanned;
use syn::{Error, Expr, ExprLit, FnArg, ItemFn, Lit, Meta, Pat, ReturnType, Type, parse2};

#[proc_macro_attribute]
pub fn ts_api(args: TokenStream, input: TokenStream) -> TokenStream {
    expand(args.into(), input.into()).into()
}

fn expand(args: TokenStream2, input: TokenStream2) -> TokenStream2 {
    let arguments = match parse_api_arguments(args) {
        Ok(arguments) => arguments,
        Err(error) => return error.to_compile_error(),
    };
    let function = match parse2::<ItemFn>(input) {
        Ok(function) => function,
        Err(error) => return error.to_compile_error(),
    };
    let parameters = match api_parameters(&function) {
        Ok(parameters) => parameters,
        Err(error) => return error.to_compile_error(),
    };
    let return_type = match api_return_type(&function.sig.output) {
        Ok(return_type) => return_type,
        Err(error) => return error.to_compile_error(),
    };
    let collection = collect_api(&function.sig.ident, arguments, parameters, return_type);
    quote! {
        #function

        #collection
    }
}

struct ApiArguments {
    method: syn::LitStr,
    path: syn::LitStr,
}

fn parse_api_arguments(args: TokenStream2) -> syn::Result<ApiArguments> {
    let metas =
        syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated.parse2(args)?;
    let mut method = None;
    let mut path = None;
    for meta in metas {
        let Meta::NameValue(value) = meta else {
            return Err(Error::new_spanned(meta, "ts_api 参数必须使用 name = value"));
        };
        let Some(name) = value.path.get_ident() else {
            return Err(Error::new_spanned(value.path, "ts_api 参数名必须是标识符"));
        };
        match name.to_string().as_str() {
            "method" => method = Some(string_literal(&value.value)?),
            "path" => path = Some(string_literal(&value.value)?),
            _ => return Err(Error::new_spanned(name, "ts_api 不支持该参数")),
        }
    }
    Ok(ApiArguments {
        method: method.ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "缺少 method"))?,
        path: path.ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "缺少 path"))?,
    })
}

fn string_literal(value: &Expr) -> syn::Result<syn::LitStr> {
    let Expr::Lit(ExprLit {
        lit: Lit::Str(value),
        ..
    }) = value
    else {
        return Err(Error::new_spanned(value, "ts_api 参数值必须是字符串"));
    };
    Ok(value.clone())
}

struct ApiParameter {
    name: syn::LitStr,
    type_name: syn::LitStr,
    kind: TokenStream2,
}

#[cfg(feature = "collect")]
fn collect_api(
    ident: &syn::Ident,
    options: ApiArguments,
    parameters: Vec<ApiParameter>,
    return_type: TokenStream2,
) -> TokenStream2 {
    let method = options.method;
    let path = options.path;
    let names = parameters.iter().map(|parameter| &parameter.name);
    let type_names = parameters.iter().map(|parameter| &parameter.type_name);
    let kinds = parameters.iter().map(|parameter| &parameter.kind);
    quote! {
        ::framework_proc_core::push_api_metadata! {
            ::framework_proc_core::ApiMetadata {
                name: stringify!(#ident),
                namespace: module_path!(),
                method: #method,
                path: #path,
                parameters: &[
                    #(::framework_proc_core::ApiParameterMetadata {
                        name: #names,
                        type_name: #type_names,
                        kind: #kinds,
                    }),*
                ],
                return_type: #return_type,
            }
        }
    }
}

#[cfg(not(feature = "collect"))]
fn collect_api(
    _: &syn::Ident,
    _: ApiArguments,
    _: Vec<ApiParameter>,
    _: TokenStream2,
) -> TokenStream2 {
    TokenStream2::new()
}

fn api_parameters(item: &ItemFn) -> syn::Result<Vec<ApiParameter>> {
    let mut parameters = Vec::new();
    for argument in &item.sig.inputs {
        let FnArg::Typed(argument) = argument else {
            return Err(Error::new_spanned(argument, "ts_api 不支持 self 参数"));
        };
        if is_type(&argument.ty, "WebContext") {
            return Err(Error::new_spanned(
                &argument.ty,
                "ts_api 不再支持 WebContext 参数，请在函数内调用 use_web()",
            ));
        }
        let (kind, inner) = if let Some(inner) = generic_type(&argument.ty, "Json") {
            (quote!(::framework_proc_core::ApiParameterKind::Body), inner)
        } else if let Some(inner) = generic_type(&argument.ty, "Query") {
            (
                quote!(::framework_proc_core::ApiParameterKind::Query),
                inner,
            )
        } else if is_type(&argument.ty, "PaginationParams") {
            (
                quote!(::framework_proc_core::ApiParameterKind::Body),
                argument.ty.as_ref(),
            )
        } else {
            return Err(Error::new_spanned(
                &argument.ty,
                "ts_api 仅支持 Json<T>、Query<T> 或 PaginationParams 参数",
            ));
        };
        let name = pattern_name(&argument.pat)?;
        let type_name = type_name(inner)?;
        parameters.push(ApiParameter {
            name: syn::LitStr::new(&name, argument.pat.span()),
            type_name: syn::LitStr::new(&type_name, inner.span()),
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
        _ => Err(Error::new_spanned(pattern, "ts_api 参数必须包含唯一绑定名")),
    }
}

fn api_return_type(output: &ReturnType) -> syn::Result<TokenStream2> {
    let ReturnType::Type(_, ty) = output else {
        return Ok(quote!(::framework_proc_core::ApiReturnType::Void));
    };
    let mut ty = ty.as_ref();
    while let Some(inner) = result_or_r_type(ty) {
        ty = inner;
    }
    if is_type(ty, "WebResponse") {
        return Ok(quote!(::framework_proc_core::ApiReturnType::Blob));
    }
    if is_unit_type(ty) {
        return Ok(quote!(::framework_proc_core::ApiReturnType::Void));
    }
    let name = syn::LitStr::new(&type_name(ty)?, ty.span());
    Ok(quote!(::framework_proc_core::ApiReturnType::Type(#name)))
}

fn result_or_r_type(ty: &Type) -> Option<&Type> {
    generic_type(ty, "Result").or_else(|| generic_type(ty, "R"))
}

fn is_type(ty: &Type, name: &str) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == name)
}

fn generic_type<'a>(ty: &'a Type, name: &str) -> Option<&'a Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != name {
        return None;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    arguments.args.iter().find_map(|argument| match argument {
        syn::GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })
}

fn type_name(ty: &Type) -> syn::Result<String> {
    let Type::Path(path) = ty else {
        return Err(Error::new_spanned(ty, "不支持该 TypeScript 类型"));
    };
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
        .ok_or_else(|| Error::new_spanned(ty, "类型路径不能为空"))
}

fn is_unit_type(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(tuple) if tuple.elems.is_empty())
}

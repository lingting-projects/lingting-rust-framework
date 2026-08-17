use proc_macro2::TokenStream;
use quote::quote;
use syn::{GenericArgument, PathArguments, ReturnType, Type};

pub fn expand_response(
    output: &ReturnType,
    function: &syn::Ident,
    arguments: &[TokenStream],
) -> TokenStream {
    match output {
        ReturnType::Default => quote! {
            #function(#(#arguments),*)
                .await;
            ::framework_web::WebResponse::empty()
        },
        ReturnType::Type(_, ty) if is_type(ty, "WebResponse") => quote! {
            #function(#(#arguments),*)
                .await
        },
        ReturnType::Type(_, ty) if is_type(ty, "R") => quote! {
            let result = #function(#(#arguments),*)
                .await;
            ::framework_web::WebResponse::from_r(result, Some(invoke_request.as_ref()))
        },
        ReturnType::Type(_, ty) => expand_typed_response(ty, function, arguments),
    }
}

fn expand_typed_response(
    ty: &Type,
    function: &syn::Ident,
    arguments: &[TokenStream],
) -> TokenStream {
    let Some(inner) = generic_type(ty, "Result") else {
        return quote! {
            let result = #function(#(#arguments),*)
                .await;
            ::framework_web::WebResponse::from_t(result, Some(invoke_request.as_ref()))
        };
    };

    if is_type(inner, "WebResponse") {
        quote! {
            let result = #function(#(#arguments),*)
                .await;
            ::framework_web::WebResponse::from_result(result, Some(invoke_request.as_ref()))
        }
    } else if is_type(inner, "R") {
        quote! {
            let result = #function(#(#arguments),*)
                .await;
            ::framework_web::WebResponse::from_result_r(result, Some(invoke_request.as_ref()))
        }
    } else {
        quote! {
            let result = #function(#(#arguments),*)
                .await;
            ::framework_web::WebResponse::from_result_t(result, Some(invoke_request.as_ref()))
        }
    }
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
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    arguments.args.iter().find_map(|argument| match argument {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })
}

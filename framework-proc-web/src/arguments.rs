use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, FnArg, ItemFn, Type};

pub fn expand_arguments(
    function: &ItemFn,
    method: crate::options::Method,
) -> syn::Result<(Vec<TokenStream>, Vec<TokenStream>)> {
    if matches!(method, crate::options::Method::Get) && function.sig.inputs.len() > 1 {
        return Err(Error::new_spanned(
            &function.sig.inputs,
            "GET 接口仅支持一个从查询参数提取的参数对象",
        ));
    }

    let mut conversions = Vec::new();
    let mut arguments = Vec::new();
    for (index, argument) in function.sig.inputs.iter().enumerate() {
        let FnArg::Typed(argument) = argument else {
            return Err(Error::new_spanned(argument, "web_api 不支持 self 参数"));
        };
        if is_type(&argument.ty, "WebContext") {
            return Err(Error::new_spanned(
                &argument.ty,
                "web_api 不再支持 WebContext 参数，请在函数内调用 use_web()",
            ));
        }

        let ty = &argument.ty;
        let variable = format_ident!("__web_argument_{index}");
        conversions.push(quote! {
            let #variable: #ty = match <#ty as ::framework_web::FromWeb>::from_web()
                .await
            {
                Ok(value) => value,
                Err(error) => return ::framework_web::WebResponse::from_error(
                    error,
                    Some(invoke_request.as_ref()),
                ),
            };
        });
        arguments.push(quote!(#variable));
    }

    Ok((conversions, arguments))
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

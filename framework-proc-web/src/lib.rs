mod arguments;
mod options;
mod response;

use crate::arguments::expand_arguments;
use crate::options::{Method, WebApiOptions};
use crate::response::expand_response;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Error, ItemFn, parse2};

#[proc_macro_attribute]
pub fn web_api(args: TokenStream, input: TokenStream) -> TokenStream {
    expand(args.into(), input.into(), None).into()
}

#[proc_macro_attribute]
pub fn web_api_get(args: TokenStream, input: TokenStream) -> TokenStream {
    expand(args.into(), input.into(), Some(Method::Get)).into()
}

#[proc_macro_attribute]
pub fn web_api_post(args: TokenStream, input: TokenStream) -> TokenStream {
    expand(args.into(), input.into(), Some(Method::Post)).into()
}

#[proc_macro_attribute]
pub fn web_api_put(args: TokenStream, input: TokenStream) -> TokenStream {
    expand(args.into(), input.into(), Some(Method::Put)).into()
}

#[proc_macro_attribute]
pub fn web_api_patch(args: TokenStream, input: TokenStream) -> TokenStream {
    expand(args.into(), input.into(), Some(Method::Patch)).into()
}

#[proc_macro_attribute]
pub fn web_api_delete(args: TokenStream, input: TokenStream) -> TokenStream {
    expand(args.into(), input.into(), Some(Method::Delete)).into()
}

fn expand(args: TokenStream2, input: TokenStream2, forced_method: Option<Method>) -> TokenStream2 {
    let options = match WebApiOptions::parse(args, forced_method) {
        Ok(options) => options,
        Err(error) => return error.to_compile_error(),
    };
    let function = match parse2::<ItemFn>(input) {
        Ok(function) => function,
        Err(error) => return error.to_compile_error(),
    };
    if function.sig.asyncness.is_none() {
        return Error::new_spanned(function.sig.fn_token, "web_api 仅支持 async fn")
            .to_compile_error();
    }

    let (conversions, call_arguments) = match expand_arguments(&function, options.method) {
        Ok(result) => result,
        Err(error) => return error.to_compile_error(),
    };
    let function_ident = &function.sig.ident;
    let route_ident = format_ident!("{}_route", function_ident);
    let method = options.method.tokens();
    let method_name = options.method.as_str();
    let path = options.path;
    let auth = options.auth.map_or_else(
        || quote!(::framework_web::AuthRule::login()),
        |value| quote!(#value),
    );
    let response = expand_response(&function.sig.output, function_ident, &call_arguments);

    quote! {
        #[::framework_proc_ts::ts_api(method = #method_name, path = #path)]
        #function

        pub(crate) fn #route_ident() -> ::framework_web::WebRoute {
            ::framework_web::WebRoute {
                method: #method,
                path: #path.to_string(),
                auth: #auth,
                invoke: ::std::sync::Arc::new(|| {
                    ::std::boxed::Box::pin(async move {
                        let context = match ::framework_web::use_web() {
                            Ok(context) => context,
                            Err(error) => return ::framework_web::WebResponse::from_error(error, None),
                        };
                        let panic_request = context
                            .request_arc();
                        let invoke_request = ::std::sync::Arc::clone(&panic_request);
                        match ::framework_web::catch_panic(async move {
                            #(#conversions)*
                            #response
                        })
                        .await
                        {
                            Ok(response) => response,
                            Err(error) => ::framework_web::WebResponse::from_error(
                                error,
                                Some(panic_request.as_ref()),
                            ),
                        }
                    })
                }),
            }
        }

    }
}

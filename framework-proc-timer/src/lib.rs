use proc_macro::TokenStream;
use quote::quote;
use syn::{Error, ItemFn, Meta, ReturnType, Token, parse::Parser};

/// 将异步函数注册为进程内定时任务。
///
/// 任务名称固定为函数名，因此同一进程内不能出现同名任务。
#[proc_macro_attribute]
pub fn timer(attribute: TokenStream, item: TokenStream) -> TokenStream {
    match expand_timer(attribute, item) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand_timer(
    attribute: TokenStream,
    item: TokenStream,
) -> Result<proc_macro2::TokenStream, Error> {
    let function = syn::parse::<ItemFn>(item)?;
    let (interval, count, retry_immediately_on_panic) = parse_arguments(attribute)?;
    validate_function(&function)?;

    let function_name = &function.sig.ident;
    let wrapper_name = syn::Ident::new(
        &format!("__timer_target_{function_name}"),
        function_name.span(),
    );
    let task_name = function_name.to_string();
    let retry_immediately_on_panic =
        retry_immediately_on_panic.map_or_else(|| quote!(None), |value| quote!(Some(#value)));

    Ok(quote! {
        #function

        fn #wrapper_name() -> ::framework_timer::TimerFuture {
            Box::pin(#function_name())
        }

        ::framework_timer::inventory::submit! {
            ::framework_timer::TimerRegistration {
                name: #task_name,
                count: #count,
                interval: #interval,
                retry_immediately_on_panic: #retry_immediately_on_panic,
                target: #wrapper_name,
            }
        }
    })
}

fn parse_arguments(attribute: TokenStream) -> Result<(i64, usize, Option<bool>), Error> {
    let arguments =
        syn::punctuated::Punctuated::<Meta, Token![,]>::parse_terminated.parse(attribute)?;
    let mut interval = None;
    let mut count = None;
    let mut retry_immediately_on_panic = None;

    for argument in arguments {
        let Meta::NameValue(argument) = argument else {
            return Err(Error::new_spanned(
                argument,
                "timer 参数必须使用 key = value 形式",
            ));
        };
        if argument.path.is_ident("interval") {
            if interval.is_some() {
                return Err(Error::new_spanned(argument, "interval 只能指定一次"));
            }
            let value = parse_integer(&argument.value, "interval 必须是毫秒整数")?;
            interval = Some(value);
        } else if argument.path.is_ident("count") {
            if count.is_some() {
                return Err(Error::new_spanned(argument, "count 只能指定一次"));
            }
            let value = parse_usize(&argument.value, "count 必须是大于等于 1 的整数")?;
            if value == 0 {
                return Err(Error::new_spanned(
                    argument.value,
                    "count 必须是大于等于 1 的整数",
                ));
            }
            count = Some(value);
        } else if argument.path.is_ident("retry_immediately_on_panic") {
            if retry_immediately_on_panic.is_some() {
                return Err(Error::new_spanned(
                    argument,
                    "retry_immediately_on_panic 只能指定一次",
                ));
            }
            retry_immediately_on_panic = Some(parse_bool(
                &argument.value,
                "retry_immediately_on_panic 必须是布尔值",
            )?);
        } else {
            return Err(Error::new_spanned(argument, "不支持的 timer 参数"));
        }
    }

    let interval = interval
        .ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "缺少必填参数 interval"))?;
    Ok((interval, count.unwrap_or(1), retry_immediately_on_panic))
}

fn parse_integer(expression: &syn::Expr, message: &str) -> Result<i64, Error> {
    let syn::Expr::Lit(expression) = expression else {
        return Err(Error::new_spanned(expression, message));
    };
    let syn::Lit::Int(value) = &expression.lit else {
        return Err(Error::new_spanned(expression, message));
    };
    value
        .base10_parse::<i64>()
        .map_err(|_| Error::new_spanned(value, message))
}

fn parse_usize(expression: &syn::Expr, message: &str) -> Result<usize, Error> {
    let syn::Expr::Lit(expression) = expression else {
        return Err(Error::new_spanned(expression, message));
    };
    let syn::Lit::Int(value) = &expression.lit else {
        return Err(Error::new_spanned(expression, message));
    };
    value
        .base10_parse::<usize>()
        .map_err(|_| Error::new_spanned(value, message))
}

fn parse_bool(expression: &syn::Expr, message: &str) -> Result<bool, Error> {
    let syn::Expr::Lit(expression) = expression else {
        return Err(Error::new_spanned(expression, message));
    };
    let syn::Lit::Bool(value) = &expression.lit else {
        return Err(Error::new_spanned(expression, message));
    };
    Ok(value.value)
}

fn validate_function(function: &ItemFn) -> Result<(), Error> {
    if function.sig.asyncness.is_none() {
        return Err(Error::new_spanned(
            &function.sig,
            "#[timer] 只能标注 async 函数",
        ));
    }
    if !function.sig.inputs.is_empty() {
        return Err(Error::new_spanned(
            &function.sig.inputs,
            "#[timer] 标注的函数不能包含参数",
        ));
    }
    match &function.sig.output {
        ReturnType::Type(_, output) if matches!(output.as_ref(), syn::Type::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "TimerResult")) =>
            {}
        _ => {
            return Err(Error::new_spanned(
                &function.sig.output,
                "#[timer] 函数必须返回 framework_timer::TimerResult",
            ));
        }
    }
    Ok(())
}

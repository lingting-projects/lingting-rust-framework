use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Error, Expr, ExprLit, Lit, Meta, Token};

#[derive(Clone, Copy)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Method {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }

    fn parse(value: &Expr) -> syn::Result<Self> {
        let Expr::Path(path) = value else {
            return Err(Error::new_spanned(value, "method 必须是 HTTP 方法标识符"));
        };
        let Some(segment) = path.path.segments.last() else {
            return Err(Error::new_spanned(value, "method 不能为空"));
        };
        let method = segment.ident.to_string().to_ascii_lowercase();
        match method.as_str() {
            "get" => Ok(Self::Get),
            "post" => Ok(Self::Post),
            "put" => Ok(Self::Put),
            "patch" => Ok(Self::Patch),
            "delete" => Ok(Self::Delete),
            _ => Err(Error::new_spanned(value, "web_api 不支持该 HTTP 方法")),
        }
    }

    pub fn tokens(self) -> TokenStream {
        match self {
            Self::Get => quote!(::framework_web::WebMethod::Get),
            Self::Post => quote!(::framework_web::WebMethod::Post),
            Self::Put => quote!(::framework_web::WebMethod::Put),
            Self::Patch => quote!(::framework_web::WebMethod::Patch),
            Self::Delete => quote!(::framework_web::WebMethod::Delete),
        }
    }
}

pub struct WebApiOptions {
    pub method: Method,
    pub path: syn::LitStr,
    pub auth: Option<Expr>,
}

impl WebApiOptions {
    pub fn parse(args: TokenStream, forced_method: Option<Method>) -> syn::Result<Self> {
        let metas = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;
        let mut method = forced_method;
        let mut path = None;
        let mut auth = None;

        for meta in metas {
            let Meta::NameValue(value) = meta else {
                return Err(Error::new_spanned(
                    meta,
                    "web_api 参数必须使用 name = value",
                ));
            };
            let Some(name) = value.path.get_ident() else {
                return Err(Error::new_spanned(value.path, "web_api 参数名必须是标识符"));
            };
            let name_value = name.to_string();
            match name_value.as_str() {
                "method" if forced_method.is_none() => method = Some(Method::parse(&value.value)?),
                "method" => {
                    return Err(Error::new_spanned(
                        value,
                        "HTTP 方法简化宏不能再次指定 method",
                    ));
                }
                "path" => path = Some(parse_path(&value.value)?),
                "auth" => auth = Some(value.value),
                _ => return Err(Error::new_spanned(name, "web_api 不支持该参数")),
            }
        }

        Ok(Self {
            method: method
                .ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "缺少 method"))?,
            path: path.ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "缺少 path"))?,
            auth,
        })
    }
}

fn parse_path(value: &Expr) -> syn::Result<syn::LitStr> {
    let Expr::Lit(ExprLit {
                      lit: Lit::Str(path),
                      ..
                  }) = value
    else {
        return Err(Error::new_spanned(value, "path 必须是字符串"));
    };
    let normalized = path.value().trim_matches('/').to_string();
    if normalized.is_empty() {
        return Err(Error::new_spanned(path, "path 不能为空"));
    }
    Ok(syn::LitStr::new(&normalized, path.span()))
}

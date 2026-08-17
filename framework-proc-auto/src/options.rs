use proc_macro2::TokenStream;
use syn::{Error, Expr, ExprLit, Lit, Meta, Token, parse::Parser, punctuated::Punctuated};

use crate::derives::DeriveOption;

#[derive(Clone, Copy)]
pub struct AutoTypeOptions {
    pub default: bool,
    pub clone: bool,
    pub copy: bool,
    pub eq: bool,
    pub serde: bool,
    pub specta: bool,
}

impl AutoTypeOptions {
    pub fn parse(args: TokenStream) -> syn::Result<Self> {
        let mut options = Self {
            default: true,
            clone: false,
            copy: false,
            eq: true,
            serde: true,
            specta: true,
        };
        parse_options(args, |name, value| {
            match name {
                "default" => options.default = value,
                "clone" => options.clone = value,
                "copy" => options.copy = value,
                "eq" => options.eq = value,
                "serde" => options.serde = value,
                "specta" => options.specta = value,
                _ => {
                    return Err(Error::new(
                        proc_macro2::Span::call_site(),
                        "auto_type 不支持该参数",
                    ));
                }
            }
            Ok(())
        })?;
        Ok(options)
    }

    pub fn derive_options(self) -> Vec<DeriveOption> {
        vec![
            DeriveOption::new("Default", self.default),
            DeriveOption::new("Debug", true),
            DeriveOption::new("Clone", self.clone || self.copy),
            DeriveOption::new("Copy", self.copy),
            DeriveOption::new("PartialEq", self.eq),
            DeriveOption::new("Eq", self.eq),
            DeriveOption::new("Serialize", self.serde),
            DeriveOption::new("Deserialize", self.serde),
            DeriveOption::new("Type", self.specta),
        ]
    }
}

#[derive(Clone, Copy)]
pub struct AutoEnumOptions {
    clone: bool,
    copy: bool,
    eq: bool,
    pub strum: bool,
    pub serde: bool,
    pub specta: bool,
}

impl AutoEnumOptions {
    pub fn parse(args: TokenStream) -> syn::Result<Self> {
        let mut options = Self {
            clone: true,
            copy: true,
            eq: true,
            strum: true,
            serde: true,
            specta: true,
        };
        parse_options(args, |name, value| {
            match name {
                "clone" => options.clone = value,
                "copy" => options.copy = value,
                "eq" => options.eq = value,
                "strum" => options.strum = value,
                "serde" => options.serde = value,
                "specta" => options.specta = value,
                _ => {
                    return Err(Error::new(
                        proc_macro2::Span::call_site(),
                        "auto_enum 不支持该参数",
                    ));
                }
            }
            Ok(())
        })?;
        Ok(options)
    }

    pub fn derive_options(self) -> Vec<DeriveOption> {
        vec![
            DeriveOption::new("Debug", true),
            DeriveOption::new("Clone", self.clone || self.copy),
            DeriveOption::new("Copy", self.copy),
            DeriveOption::new("PartialEq", self.eq),
            DeriveOption::new("Eq", self.eq),
            DeriveOption::new("EnumIter", self.strum),
            DeriveOption::new("EnumString", self.strum),
            DeriveOption::new("Display", self.strum),
            DeriveOption::new("Serialize", self.serde),
            DeriveOption::new("Deserialize", self.serde),
            DeriveOption::new("Type", self.specta),
        ]
    }
}

fn parse_options(
    args: TokenStream,
    mut apply: impl FnMut(&str, bool) -> syn::Result<()>,
) -> syn::Result<()> {
    let metas = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;
    for meta in metas {
        let Meta::NameValue(name_value) = meta else {
            return Err(Error::new_spanned(
                meta,
                "参数必须采用 name = true 或 name = false 形式",
            ));
        };
        let Some(ident) = name_value.path.get_ident() else {
            return Err(Error::new_spanned(name_value.path, "参数名称必须是标识符"));
        };
        let Expr::Lit(ExprLit {
                          lit: Lit::Bool(value),
                          ..
                      }) = name_value.value
        else {
            return Err(Error::new_spanned(name_value, "参数值必须是布尔值"));
        };
        apply(&ident.to_string(), value.value)?;
    }
    Ok(())
}

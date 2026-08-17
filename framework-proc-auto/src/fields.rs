use proc_macro2::TokenStream;
use quote::quote;
use syn::{Field, Token, punctuated::Punctuated};

use crate::attributes::{ensure_serde_adapter, ensure_serde_default, ensure_specta_type};
use crate::type_transform::{TypeTransform, TypeWrapper};

pub struct FieldSummary {
    pub requires_serde_as: bool,
}

pub fn transform_fields(
    fields: &mut Punctuated<Field, Token![,]>,
    serde_enabled: bool,
    specta_enabled: bool,
) -> FieldSummary {
    let mut summary = FieldSummary {
        requires_serde_as: false,
    };

    for field in fields {
        let transform = TypeTransform::from_type(&field.ty);
        if serde_enabled && transform.is_option {
            ensure_serde_default(&mut field.attrs);
        }
        if !transform.requires_string {
            continue;
        }
        if serde_enabled {
            ensure_serde_adapter(&mut field.attrs, serde_adapter(transform.wrapper));
            summary.requires_serde_as = true;
        }
        if specta_enabled {
            ensure_specta_type(&mut field.attrs, specta_type(transform.wrapper));
        }
    }

    summary
}

fn serde_adapter(wrapper: TypeWrapper) -> &'static str {
    match wrapper {
        TypeWrapper::Plain => "serde_with::DisplayFromStr",
        TypeWrapper::Option => "Option<serde_with::DisplayFromStr>",
        TypeWrapper::Vec => "Vec<serde_with::DisplayFromStr>",
        TypeWrapper::OptionVec => "Option<Vec<serde_with::DisplayFromStr>>",
    }
}

fn specta_type(wrapper: TypeWrapper) -> TokenStream {
    match wrapper {
        TypeWrapper::Plain => quote!(String),
        TypeWrapper::Option => quote!(Option<String>),
        TypeWrapper::Vec => quote!(Vec<String>),
        TypeWrapper::OptionVec => quote!(Option<Vec<String>>),
    }
}

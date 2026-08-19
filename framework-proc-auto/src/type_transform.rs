use proc_macro2::TokenStream;
use quote::quote;
use syn::{GenericArgument, PathArguments, Type};

/// Rust 类型在序列化和 TypeScript 导出时的转换方式。
pub struct TypeTransform {
    pub is_option: bool,
    pub serde_adapter: Option<TokenStream>,
    pub specta_type: Option<TokenStream>,
    enum_value: Option<EnumValueTransform>,
}

impl TypeTransform {
    pub fn from_type(ty: &Type) -> Self {
        if let Some(inner) = generic_inner_type(ty, "Option") {
            let transform = special_type(inner);
            return Self {
                is_option: true,
                serde_adapter: transform.serde_adapter.map(|ty| quote!(Option<#ty>)),
                specta_type: transform.specta_type.map(|ty| quote!(Option<#ty>)),
                enum_value: transform.enum_value.map(EnumValueTransform::option),
            };
        }

        special_type(ty).into_transform()
    }

    pub fn enum_value(&self, method: &syn::Ident) -> Option<TokenStream> {
        self.enum_value.map(|transform| transform.tokens(method))
    }
}

#[derive(Clone, Copy)]
enum EnumValueTransform {
    Plain,
    Option,
    Vec,
    OptionVec,
}

impl EnumValueTransform {
    fn option(self) -> Self {
        match self {
            Self::Plain => Self::Option,
            Self::Vec => Self::OptionVec,
            Self::Option | Self::OptionVec => self,
        }
    }

    fn vec(self) -> Self {
        match self {
            Self::Plain => Self::Vec,
            Self::Option => Self::OptionVec,
            Self::Vec | Self::OptionVec => self,
        }
    }

    fn tokens(self, method: &syn::Ident) -> TokenStream {
        match self {
            Self::Plain => quote!(::framework_proc_core::serde_json::Value::String(
                value.#method().to_string(),
            )),
            Self::Option => quote!(match value.#method() {
                Some(value) => ::framework_proc_core::serde_json::Value::String(value.to_string()),
                None => ::framework_proc_core::serde_json::Value::Null,
            }),
            Self::Vec => quote!(::framework_proc_core::serde_json::Value::Array(
                value.#method()
                    .into_iter()
                    .map(|value| ::framework_proc_core::serde_json::Value::String(value.to_string()))
                    .collect(),
            )),
            Self::OptionVec => quote!(match value.#method() {
                Some(values) => ::framework_proc_core::serde_json::Value::Array(
                    values
                        .into_iter()
                        .map(|value| ::framework_proc_core::serde_json::Value::String(value.to_string()))
                        .collect(),
                ),
                None => ::framework_proc_core::serde_json::Value::Null,
            }),
        }
    }
}

struct SpecialTypeTransform {
    serde_adapter: Option<TokenStream>,
    specta_type: Option<TokenStream>,
    enum_value: Option<EnumValueTransform>,
}

impl SpecialTypeTransform {
    const fn unchanged() -> Self {
        Self {
            serde_adapter: None,
            specta_type: None,
            enum_value: None,
        }
    }

    fn into_transform(self) -> TypeTransform {
        TypeTransform {
            is_option: false,
            serde_adapter: self.serde_adapter,
            specta_type: self.specta_type,
            enum_value: self.enum_value,
        }
    }
}

fn special_type(ty: &Type) -> SpecialTypeTransform {
    if is_string_serialized_type(ty) {
        return SpecialTypeTransform {
            serde_adapter: Some(quote!(serde_with::DisplayFromStr)),
            specta_type: Some(quote!(String)),
            enum_value: Some(EnumValueTransform::Plain),
        };
    }
    if is_json_value(ty) {
        return SpecialTypeTransform {
            serde_adapter: None,
            specta_type: Some(quote!(specta_typescript::Unknown)),
            enum_value: None,
        };
    }
    if let Some(inner) = generic_inner_type(ty, "Vec") {
        return vec_transform(inner);
    }
    if let Some((kind, key, value)) = map_types(ty) {
        return map_transform(kind, key, value);
    }

    SpecialTypeTransform::unchanged()
}

fn vec_transform(inner: &Type) -> SpecialTypeTransform {
    let transform = special_type(inner);
    SpecialTypeTransform {
        serde_adapter: transform.serde_adapter.map(|ty| quote!(Vec<#ty>)),
        specta_type: transform.specta_type.map(|ty| quote!(Vec<#ty>)),
        enum_value: transform.enum_value.map(EnumValueTransform::vec),
    }
}

fn map_transform(kind: MapKind, key: &Type, value: &Type) -> SpecialTypeTransform {
    let key_transform = special_type(key);
    let value_transform = special_type(value);
    let serde_adapter = match (&key_transform.serde_adapter, &value_transform.serde_adapter) {
        (None, None) => None,
        (key_adapter, value_adapter) => {
            let key = key_adapter
                .as_ref()
                .map_or_else(|| quote!(#key), Clone::clone);
            let value = value_adapter
                .as_ref()
                .map_or_else(|| quote!(#value), Clone::clone);
            Some(kind.tokens(quote!(#key, #value)))
        }
    };
    let specta_type =
        if key_transform.specta_type.is_some() || value_transform.specta_type.is_some() {
            let key = key_transform.specta_type.unwrap_or_else(|| quote!(#key));
            let value = value_transform
                .specta_type
                .unwrap_or_else(|| quote!(#value));
            Some(kind.tokens(quote!(#key, #value)))
        } else {
            None
        };

    SpecialTypeTransform {
        serde_adapter,
        specta_type,
        enum_value: None,
    }
}

#[derive(Clone, Copy)]
enum MapKind {
    Hash,
    BTree,
}

impl MapKind {
    fn tokens(self, types: TokenStream) -> TokenStream {
        match self {
            Self::Hash => quote!(std::collections::HashMap<#types>),
            Self::BTree => quote!(std::collections::BTreeMap<#types>),
        }
    }
}

fn is_string_serialized_type(ty: &Type) -> bool {
    let Some(name) = type_last_segment(ty) else {
        return false;
    };
    ["u64", "i64", "u128", "i128", "Uuid", "Decimal", "DateTime"].contains(&name.as_str())
}

fn is_json_value(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if path.path.is_ident("serde_json::Value"))
        || type_last_segment(ty).is_some_and(|name| name == "Value")
}

fn map_types(ty: &Type) -> Option<(MapKind, &Type, &Type)> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    let kind = match segment.ident.to_string().as_str() {
        "HashMap" | "Map" => MapKind::Hash,
        "BTreeMap" => MapKind::BTree,
        _ => return None,
    };
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let mut types = arguments.args.iter().filter_map(|argument| match argument {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    });
    Some((kind, types.next()?, types.next()?))
}

fn generic_inner_type<'a>(ty: &'a Type, name: &str) -> Option<&'a Type> {
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
        GenericArgument::Type(inner) => Some(inner),
        _ => None,
    })
}

fn type_last_segment(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

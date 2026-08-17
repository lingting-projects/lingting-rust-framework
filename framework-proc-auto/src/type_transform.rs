use syn::Type;

/// Rust 类型在 TypeScript 导出时的转换方式。
pub struct TypeTransform {
    pub is_option: bool,
    pub wrapper: TypeWrapper,
    pub requires_string: bool,
}

impl TypeTransform {
    pub fn from_type(ty: &Type) -> Self {
        if let Some(inner) = generic_inner_type(ty, "Option") {
            if let Some(inner) = generic_inner_type(inner, "Vec") {
                return Self {
                    is_option: true,
                    wrapper: TypeWrapper::OptionVec,
                    requires_string: requires_string(inner),
                };
            }
            return Self {
                is_option: true,
                wrapper: TypeWrapper::Option,
                requires_string: requires_string(inner),
            };
        }
        if let Some(inner) = generic_inner_type(ty, "Vec") {
            return Self {
                is_option: false,
                wrapper: TypeWrapper::Vec,
                requires_string: requires_string(inner),
            };
        }
        Self {
            is_option: false,
            wrapper: TypeWrapper::Plain,
            requires_string: requires_string(ty),
        }
    }
}

#[derive(Clone, Copy)]
pub enum TypeWrapper {
    Plain,
    Option,
    Vec,
    OptionVec,
}

fn requires_string(ty: &Type) -> bool {
    let Some(name) = type_last_segment(ty) else {
        return false;
    };
    ["u64", "i64", "u128", "i128", "Uuid", "Decimal", "DateTime"].contains(&name.as_str())
}

fn generic_inner_type<'a>(ty: &'a Type, name: &str) -> Option<&'a Type> {
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
        syn::GenericArgument::Type(inner) => Some(inner),
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

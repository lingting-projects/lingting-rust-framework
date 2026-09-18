use std::collections::BTreeSet;

use crate::{ApiMetadata, ApiReturnType};

pub(crate) fn collect_api_type_names(apis: &[ApiMetadata]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for api in apis {
        for parameter in api.parameters {
            collect_type_names(parameter.type_name, &mut names);
        }
        if let ApiReturnType::Type(name) = api.return_type {
            collect_type_names(name, &mut names);
        }
    }
    names
}

pub(crate) fn collect_type_names(value: &str, names: &mut BTreeSet<String>) {
    for name in
        value.split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
    {
        if !name.is_empty()
            && !matches!(
                name,
                "string"
                    | "number"
                    | "boolean"
                    | "void"
                    | "blob"
                    | "unknown"
                    | "null"
                    | "Array"
                    | "Record"
            )
        {
            names.insert(name.to_string());
        }
    }
}

pub(crate) fn camel_case(name: &str) -> String {
    let mut result = String::new();
    let mut uppercase_next = false;
    for (index, character) in name.chars().enumerate() {
        if character == '_' || character == '-' {
            uppercase_next = !result.is_empty();
            continue;
        }
        if index == 0 {
            result.extend(character.to_lowercase());
        } else if uppercase_next {
            result.extend(character.to_uppercase());
            uppercase_next = false;
        } else {
            result.push(character);
        }
    }
    result
}

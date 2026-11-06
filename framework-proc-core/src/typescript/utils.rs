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

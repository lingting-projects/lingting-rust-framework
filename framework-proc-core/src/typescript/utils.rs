use std::collections::BTreeSet;

use crate::{ApiMetadata, ApiReturnKind};

pub(crate) fn collect_api_type_names(apis: &[ApiMetadata]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for api in apis {
        for parameter in api.parameters {
            collect_type_names(parameter.ts_type_name(), &mut names);
        }
        if api.return_type.kind == ApiReturnKind::Type {
            collect_type_names(
                api.return_type.ts_type_name().unwrap_or("unknown"),
                &mut names,
            );
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

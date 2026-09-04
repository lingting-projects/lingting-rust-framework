use crate::{ApiMetadata, ApiParameterKind, ApiReturnType};

use super::utils::camel_case;

pub(crate) fn declaration(class_name: &str, apis: &[ApiMetadata]) -> String {
    let methods = apis
        .iter()
        .map(declaration_method)
        .collect::<Vec<_>>()
        .join("\n\n");
    let method_block = if methods.is_empty() {
        String::new()
    } else {
        format!("\n\n{methods}")
    };
    format!(
        "export declare abstract class {class_name} {{\n  protected abstract call<T>(method: string, path: string, body?: any, query?: any): Promise<T>;{method_block}\n}}\n"
    )
}

pub(crate) fn javascript(class_name: &str, apis: &[ApiMetadata]) -> String {
    let methods = apis
        .iter()
        .map(javascript_method)
        .collect::<Vec<_>>()
        .join("\n\n");
    let method_block = if methods.is_empty() {
        String::new()
    } else {
        format!("\n\n{methods}\n")
    };
    format!("export class {class_name} {{{method_block}}}\n")
}

fn declaration_method(api: &ApiMetadata) -> String {
    let parameters = api
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, parameter.type_name))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "  {}({parameters}): Promise<{}>;",
        camel_case(api.name),
        return_type_name(api.return_type),
    )
}

fn javascript_method(api: &ApiMetadata) -> String {
    let parameters = api
        .parameters
        .iter()
        .map(|parameter| parameter.name)
        .collect::<Vec<_>>()
        .join(", ");
    let body = api
        .parameters
        .iter()
        .filter(|parameter| parameter.kind == ApiParameterKind::Body)
        .map(|parameter| parameter.name)
        .collect::<Vec<_>>();
    let query = api
        .parameters
        .iter()
        .filter(|parameter| parameter.kind == ApiParameterKind::Query)
        .map(|parameter| parameter.name)
        .collect::<Vec<_>>();
    let call_arguments = match (body, query) {
        (body, query) if body.is_empty() && query.is_empty() => String::new(),
        (body, query) if query.is_empty() => format!(", {}", request_value(&body)),
        (body, query) if body.is_empty() => format!(", undefined, {}", request_value(&query)),
        (body, query) => format!(", {}, {}", request_value(&body), request_value(&query)),
    };
    format!(
        "  {}({parameters}) {{\n    return this.call(\"{}\", \"{}\"{});\n  }}",
        camel_case(api.name),
        api.method,
        api.path,
        call_arguments,
    )
}

fn request_value(parameters: &[&str]) -> String {
    if parameters.len() == 1 {
        return parameters[0].to_string();
    }
    format!(
        "{{ {} }}",
        parameters
            .iter()
            .map(|name| format!("...{name}"))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn return_type_name(return_type: ApiReturnType) -> &'static str {
    match return_type {
        ApiReturnType::Void => "void",
        ApiReturnType::Blob => "blob",
        ApiReturnType::Type(name) => name,
    }
}

use super::utils::camel_case;
use crate::ApiMetadata;
use crate::typescript::common::return_type_name;

pub(crate) fn declaration(apis: &[ApiMetadata]) -> String {
    format!(
        "export type ApiDefinition<TArgs extends readonly unknown[], TResult> = {{\n  readonly method: string;\n  readonly path: string;\n  readonly fn: (...args: TArgs) => TResult;\n}};\n\n{}\n\nexport type ApiName = keyof typeof ApiDefinitions;\n\nexport type ApiArgs<TName extends ApiName> = Parameters<typeof ApiDefinitions[TName][\"fn\"]>;\n\nexport type ApiResult<TName extends ApiName> = ReturnType<typeof ApiDefinitions[TName][\"fn\"]>;\n\nexport type ApiArg<TName extends ApiName, TIndex extends number> = ApiArgs<TName>[TIndex];\n",
        declaration_definitions(apis),
    )
}

pub(crate) fn javascript(apis: &[ApiMetadata]) -> String {
    let definitions = apis
        .iter()
        .map(javascript_definition)
        .collect::<Vec<_>>()
        .join(",\n");
    let definition_block = if definitions.is_empty() {
        String::new()
    } else {
        format!("\n{definitions}\n")
    };
    format!("export const ApiDefinitions = {{{definition_block}}};\n")
}

fn declaration_definitions(apis: &[ApiMetadata]) -> String {
    let definitions = apis
        .iter()
        .map(declaration_definition)
        .collect::<Vec<_>>()
        .join("\n");
    let definition_block = if definitions.is_empty() {
        String::new()
    } else {
        format!("\n{definitions}\n")
    };
    format!("export declare const ApiDefinitions: {{{definition_block}}};")
}

fn declaration_definition(api: &ApiMetadata) -> String {
    let parameters = api.parameters_colon().join(", ");
    format!(
        "  readonly {}: ApiDefinition<[{parameters}], {}>;",
        camel_case(api.name),
        return_type_name(api.return_type),
    )
}

fn javascript_definition(api: &ApiMetadata) -> String {
    let parameters = api
        .parameters
        .iter()
        .map(|parameter| parameter.name)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "  {}: {{ method: \"{}\", path: \"{}\", fn: ({parameters}) => null }}",
        camel_case(api.name),
        api.method,
        api.path,
    )
}

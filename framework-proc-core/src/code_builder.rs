use std::borrow::Borrow;
use std::collections::{BTreeSet, HashMap};
use std::io;

use specta::Types;
use specta_serde::PhasesFormat;
use specta_typescript::Typescript;

use crate::{ApiMetadata, ApiParameterKind, ApiReturnType, EnumMetadata, EnumValue, TypeMetadata};

pub type TypeScriptResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// TypeScript 声明和运行时模块的内存内容。
pub struct GeneratedTypeScript {
    pub javascript: String,
    pub declaration: String,
}

/// 根据收集到的元数据构造 TypeScript 文件内容。
pub struct TypeScriptCodeBuilder {
    type_metadata: Vec<TypeMetadata>,
    enum_metadata: Vec<EnumMetadata>,
    api_metadata: Vec<ApiMetadata>,
    api_class_name: Option<String>,
    api_type_import_from: Option<String>,
}

impl TypeScriptCodeBuilder {
    pub fn new() -> Self {
        Self {
            type_metadata: Vec::new(),
            enum_metadata: Vec::new(),
            api_metadata: Vec::new(),
            api_class_name: None,
            api_type_import_from: None,
        }
    }

    pub fn types<I, M>(mut self, metadata: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Borrow<TypeMetadata>,
    {
        self.push_types(metadata);
        self
    }

    pub fn apis<I, M>(mut self, metadata: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Borrow<ApiMetadata>,
    {
        self.push_apis(metadata);
        self
    }

    pub fn enums<I, M>(mut self, metadata: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Borrow<EnumMetadata>,
    {
        self.push_enums(metadata);
        self
    }

    pub fn api_class_name(mut self, name: impl Into<String>) -> Self {
        self.api_class_name = Some(name.into());
        self
    }

    pub fn api_type_import_from(mut self, package_name: impl Into<String>) -> Self {
        self.api_type_import_from = Some(package_name.into());
        self
    }

    pub fn push_types<I, M>(&mut self, metadata: I)
    where
        I: IntoIterator<Item = M>,
        M: Borrow<TypeMetadata>,
    {
        self.type_metadata
            .extend(metadata.into_iter().map(|item| *item.borrow()));
    }

    pub fn push_apis<I, M>(&mut self, metadata: I)
    where
        I: IntoIterator<Item = M>,
        M: Borrow<ApiMetadata>,
    {
        self.api_metadata
            .extend(metadata.into_iter().map(|item| *item.borrow()));
    }

    pub fn push_enums<I, M>(&mut self, metadata: I)
    where
        I: IntoIterator<Item = M>,
        M: Borrow<EnumMetadata>,
    {
        self.enum_metadata
            .extend(metadata.into_iter().map(|item| *item.borrow()));
    }

    pub fn build(&self) -> TypeScriptResult<GeneratedTypeScript> {
        let class_name = self.required_api_class_name()?;
        validate_api_metadata(&self.api_metadata)?;
        validate_enum_metadata(&self.type_metadata, &self.enum_metadata)?;
        let type_declaration = self.type_declaration()?;
        let enum_declaration = enum_declaration(&self.enum_metadata)?;
        let type_import = self.type_import()?;
        let declaration = format!(
            "{}{}{}{}",
            type_import,
            type_declaration,
            enum_declaration,
            declaration_class(class_name, &self.api_metadata),
        );

        Ok(GeneratedTypeScript {
            javascript: format!(
                "{}{}",
                javascript_enums(&self.enum_metadata)?,
                javascript_class(class_name, &self.api_metadata),
            ),
            declaration,
        })
    }

    fn required_api_class_name(&self) -> TypeScriptResult<&str> {
        let Some(name) = self.api_class_name.as_deref() else {
            return Err(build_error("缺少 TypeScript API 抽象类名"));
        };
        if !is_typescript_identifier(name) {
            return Err(build_error("TypeScript API 抽象类名必须是有效标识符"));
        }
        Ok(name)
    }

    fn type_declaration(&self) -> TypeScriptResult<String> {
        let mut types = Types::default();
        for metadata in &self.type_metadata {
            types = metadata.register_type(types);
        }
        Ok(Typescript::default().export(&types, PhasesFormat)?)
    }

    fn type_import(&self) -> TypeScriptResult<String> {
        let Some(package_name) = self.api_type_import_from.as_deref() else {
            return Ok(String::new());
        };
        if package_name.is_empty() {
            return Err(build_error("TypeScript API 类型导入包名不能为空"));
        }

        let imported = api_type_names(&self.api_metadata);
        if imported.is_empty() {
            return Ok(String::new());
        }
        let local = self
            .type_metadata
            .iter()
            .map(|metadata| type_short_name(metadata.full_name()))
            .collect::<BTreeSet<_>>();
        let conflicts = imported.intersection(&local).cloned().collect::<Vec<_>>();
        if !conflicts.is_empty() {
            return Err(build_error(format!(
                "API 类型导入与本地导出类型重名: {}",
                conflicts.join(", ")
            )));
        }

        Ok(format!(
            "import type {{ {} }} from \"{}\";\n\n",
            imported.into_iter().collect::<Vec<_>>().join(", "),
            package_name,
        ))
    }
}

fn validate_enum_metadata(types: &[TypeMetadata], enums: &[EnumMetadata]) -> TypeScriptResult<()> {
    let type_names = types
        .iter()
        .map(|metadata| type_short_name(metadata.full_name()))
        .collect::<BTreeSet<_>>();
    let mut enum_names = BTreeSet::new();
    for metadata in enums {
        let name = type_short_name(metadata.full_name());
        if !type_names.contains(&name) {
            return Err(build_error(format!("枚举运行时导出缺少类型声明: {name}")));
        }
        if !enum_names.insert(name.to_string()) {
            return Err(build_error(format!("枚举运行时导出重复: {name}")));
        }

        let mut field_names = BTreeSet::new();
        for field in metadata.fields {
            if !field_names.insert(*field) {
                return Err(build_error(format!("枚举 {name} 的导出字段重复: {field}")));
            }
        }

        let mut values = BTreeSet::new();
        for value in metadata.values()? {
            if !values.insert(value.value.clone()) {
                return Err(build_error(format!(
                    "枚举 {name} 的导出值重复: {}",
                    value.value
                )));
            }
            let actual_fields = value
                .fields
                .iter()
                .map(|(field, _)| *field)
                .collect::<BTreeSet<_>>();
            if actual_fields.len() != value.fields.len()
                || actual_fields.len() != metadata.fields.len()
                || !metadata
                    .fields
                    .iter()
                    .all(|field| actual_fields.contains(field))
            {
                return Err(build_error(format!("枚举 {name} 的导出字段与元数据不一致")));
            }
        }
    }
    Ok(())
}

fn enum_declaration(enums: &[EnumMetadata]) -> TypeScriptResult<String> {
    let mut result = String::new();
    for metadata in enums {
        let name = type_short_name(metadata.full_name());
        let values = metadata.values()?;
        let fields = metadata
            .fields
            .iter()
            .map(|field| {
                let types = values
                    .iter()
                    .filter_map(|value| {
                        value
                            .fields
                            .iter()
                            .find(|(name, _)| name == field)
                            .map(|(_, value)| typescript_value_type(value))
                    })
                    .collect::<std::collections::BTreeSet<_>>();
                let type_name = types.into_iter().collect::<Vec<_>>();
                let type_name = if type_name.is_empty() {
                    "unknown".to_string()
                } else {
                    type_name.join(" | ")
                };
                format!("{}: {type_name};", typescript_property(field))
            })
            .collect::<Vec<_>>()
            .join(" ");
        let record = if fields.is_empty() {
            format!("{{ value: {name}; }}")
        } else {
            format!("{{ value: {name}; {fields} }}")
        };
        result.push_str(&format!(
            "\nexport declare const {name}All: readonly {name}[];\nexport declare const {name}Map: Readonly<Record<{name}, {record}>>;\n"
        ));
    }
    Ok(result)
}

fn javascript_enums(enums: &[EnumMetadata]) -> TypeScriptResult<String> {
    let mut result = String::new();
    for metadata in enums {
        let name = type_short_name(metadata.full_name());
        let values = metadata.values()?;
        let all = values
            .iter()
            .map(|value| json_string(&value.value))
            .collect::<TypeScriptResult<Vec<_>>>()?
            .join(", ");
        let map = values
            .iter()
            .map(javascript_enum_value)
            .collect::<TypeScriptResult<Vec<_>>>()?
            .join(", ");
        result.push_str(&format!(
            "export const {name}All = [{all}];\nexport const {name}Map = {{ {map} }};\n\n"
        ));
    }
    Ok(result)
}

fn javascript_enum_value(value: &EnumValue) -> TypeScriptResult<String> {
    let mut properties = vec![format!("value: {}", json_string(&value.value)?)];
    for (field, field_value) in &value.fields {
        properties.push(format!(
            "{}: {}",
            javascript_property(field),
            serde_json::to_string(field_value)?,
        ));
    }
    Ok(format!(
        "{}: {{ {} }}",
        json_string(&value.value)?,
        properties.join(", "),
    ))
}

fn typescript_value_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "unknown[]",
        serde_json::Value::Object(_) => "Record<string, unknown>",
    }
}

fn json_string(value: &str) -> TypeScriptResult<String> {
    serde_json::to_string(value).map_err(|error| Box::new(error) as _)
}

fn typescript_property(name: &str) -> String {
    if is_typescript_identifier(name) {
        name.to_string()
    } else {
        serde_json::to_string(name).unwrap_or_else(|_| format!("\"{name}\""))
    }
}

fn javascript_property(name: &str) -> String {
    if is_typescript_identifier(name) {
        name.to_string()
    } else {
        serde_json::to_string(name).unwrap_or_else(|_| format!("\"{name}\""))
    }
}

impl Default for TypeScriptCodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_api_metadata(apis: &[ApiMetadata]) -> TypeScriptResult<()> {
    let mut names = HashMap::<String, Vec<&ApiMetadata>>::new();
    let mut routes = HashMap::<String, Vec<&ApiMetadata>>::new();
    for api in apis {
        names.entry(camel_case(api.name)).or_default().push(api);
        routes
            .entry(format!("{} {}", api.method, api.path))
            .or_default()
            .push(api);
    }

    let mut problems = Vec::new();
    for (name, duplicates) in names.into_iter().filter(|(_, values)| values.len() > 1) {
        problems.push(format!(
            "方法名重复 {name}: {}",
            duplicate_sources(&duplicates)
        ));
    }
    for (route, duplicates) in routes.into_iter().filter(|(_, values)| values.len() > 1) {
        problems.push(format!(
            "请求方法和地址重复 {route}: {}",
            duplicate_sources(&duplicates)
        ));
    }
    if problems.is_empty() {
        Ok(())
    } else {
        problems.sort();
        Err(build_error(format!(
            "TypeScript API 导出失败:\n{}",
            problems.join("\n")
        )))
    }
}

fn duplicate_sources(apis: &[&ApiMetadata]) -> String {
    apis.iter()
        .map(|api| format!("{}::{}", api.namespace, api.name))
        .collect::<Vec<_>>()
        .join(", ")
}

fn declaration_class(class_name: &str, apis: &[ApiMetadata]) -> String {
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
        "\nexport declare abstract class {class_name} {{\n  protected abstract call<T>(method: string, path: string, body?: any, query?: any): Promise<T>;{method_block}\n}}\n"
    )
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

fn javascript_class(class_name: &str, apis: &[ApiMetadata]) -> String {
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
            .join(", ")
    )
}

fn return_type_name(return_type: ApiReturnType) -> &'static str {
    match return_type {
        ApiReturnType::Void => "void",
        ApiReturnType::Blob => "blob",
        ApiReturnType::Type(name) => name,
    }
}

fn api_type_names(apis: &[ApiMetadata]) -> BTreeSet<String> {
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

fn collect_type_names(value: &str, names: &mut BTreeSet<String>) {
    for name in
        value.split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
    {
        if !name.is_empty()
            && !matches!(
                name,
                "string" | "number" | "boolean" | "void" | "blob" | "unknown" | "null" | "Array"
            )
        {
            names.insert(name.to_string());
        }
    }
}

fn type_short_name(name: &str) -> String {
    name.rsplit("::").next().unwrap_or(name).to_string()
}

fn camel_case(name: &str) -> String {
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

fn is_typescript_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    matches!(characters.next(), Some(value) if value.is_ascii_alphabetic() || value == '_')
        && characters.all(|value| value.is_ascii_alphanumeric() || value == '_')
}

fn build_error(message: impl Into<String>) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(io::Error::other(message.into()))
}

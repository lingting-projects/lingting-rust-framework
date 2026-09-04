use std::borrow::Borrow;
use std::collections::HashMap;

use crate::{ApiMetadata, ApiParameterKind, ApiReturnType};

use super::common::{build_error, is_typescript_identifier};
use super::utils::{camel_case, collect_api_type_names};
use super::{TypescriptBuildResult, TypescriptBuilder, TypescriptResult};

/// 构建 API 抽象类及其类型导入。
pub struct TypescriptApiBuilder {
    metadata: Vec<ApiMetadata>,
    class_name: Option<String>,
    type_import_from: String,
}

impl TypescriptApiBuilder {
    pub fn new() -> Self {
        Self {
            metadata: Vec::new(),
            class_name: None,
            type_import_from: ".".to_string(),
        }
    }

    pub fn apis<I, M>(mut self, metadata: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Borrow<ApiMetadata>,
    {
        self.push_apis(metadata);
        self
    }

    pub fn class_name(mut self, name: impl Into<String>) -> Self {
        self.class_name = Some(name.into());
        self
    }

    pub fn type_import_from(mut self, source: impl Into<String>) -> Self {
        self.type_import_from = source.into();
        self
    }

    pub fn push_apis<I, M>(&mut self, metadata: I)
    where
        I: IntoIterator<Item = M>,
        M: Borrow<ApiMetadata>,
    {
        self.metadata
            .extend(metadata.into_iter().map(|item| *item.borrow()));
    }
}

impl TypescriptBuilder for TypescriptApiBuilder {
    fn build(&self) -> TypescriptResult<TypescriptBuildResult> {
        let class_name = self.required_class_name()?;
        validate_api_metadata(&self.metadata)?;
        Ok(TypescriptBuildResult {
            js: javascript_class(class_name, &self.metadata),
            dts: format!(
                "{}{}",
                type_import(&self.metadata, &self.type_import_from)?,
                declaration_class(class_name, &self.metadata),
            ),
        })
    }
}

impl Default for TypescriptApiBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TypescriptApiBuilder {
    fn required_class_name(&self) -> TypescriptResult<&str> {
        let Some(name) = self.class_name.as_deref() else {
            return Err(build_error("缺少 TypeScript API 抽象类名"));
        };
        if !is_typescript_identifier(name) {
            return Err(build_error("TypeScript API 抽象类名必须是有效标识符"));
        }
        Ok(name)
    }
}

fn type_import(apis: &[ApiMetadata], source: &str) -> TypescriptResult<String> {
    if source.is_empty() {
        return Err(build_error("TypeScript API 类型导入来源不能为空"));
    }
    let names = collect_api_type_names(apis);
    if names.is_empty() {
        return Ok(String::new());
    }
    Ok(format!(
        "import type {{ {} }} from \"{}\";\n\n",
        names.into_iter().collect::<Vec<_>>().join(", "),
        source,
    ))
}

fn validate_api_metadata(apis: &[ApiMetadata]) -> TypescriptResult<()> {
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
        "export declare abstract class {class_name} {{\n  protected abstract call<T>(method: string, path: string, body?: any, query?: any): Promise<T>;{method_block}\n}}\n"
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

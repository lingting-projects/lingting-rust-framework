use std::collections::BTreeSet;

use crate::utils::shouty_snake_case;
use crate::{ApiMetadata, ApiParameterKind, ApiReturnKind};

use super::common::{RustResult, build_error, string_literal};
use super::utils::collect_type_names;

/// 生成 `ApiArgumentKind`、`ApiArgument`、`ApiDefinition` 与各接口常量。
pub(crate) fn declaration(apis: &[ApiMetadata]) -> RustResult<String> {
    let mut definitions = Vec::new();
    let mut constants = Vec::new();
    for api in apis {
        let constant = shouty_snake_case(api.name);
        let arguments = api
            .parameters
            .iter()
            .map(|parameter| {
                format!(
                    "ApiArgument {{ name: {}, type_name: {}, kind: ApiArgumentKind::{} }}",
                    string_literal(parameter.name),
                    string_literal(parameter.rust_type_name()),
                    kind_name(parameter.kind),
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        definitions.push(format!(
            "/// {doc}\npub const {constant}: ApiDefinition = ApiDefinition {{\n    name: {name},\n    namespace: {namespace},\n    method: {method},\n    path: {path},\n    arguments: &[{arguments}],\n    return_type: {return_type},\n}};",
            doc = api.name,
            name = string_literal(api.name),
            namespace = string_literal(api.namespace),
            method = string_literal(api.method),
            path = string_literal(api.path),
            return_type = string_literal(return_type_name(api)?),
        ));
        constants.push(constant);
    }
    Ok(format!(
        "{HEADER}\n\n{}\n\n/// 全部接口定义。\npub const API_DEFINITIONS: &[ApiDefinition] = &[{}];",
        definitions.join("\n\n"),
        constants.join(", "),
    ))
}

const HEADER: &str = r#"/// 接口参数的传输位置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiArgumentKind {
    Body,
    Query,
}

/// 接口参数描述。
pub struct ApiArgument {
    pub name: &'static str,
    pub type_name: &'static str,
    pub kind: ApiArgumentKind,
}

/// 接口定义。
pub struct ApiDefinition {
    pub name: &'static str,
    pub namespace: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub arguments: &'static [ApiArgument],
    pub return_type: &'static str,
}"#;

pub(crate) fn return_type_name(api: &ApiMetadata) -> RustResult<&'static str> {
    match api.return_type.kind {
        ApiReturnKind::Void => Ok("()"),
        ApiReturnKind::Blob => Ok("Vec<u8>"),
        ApiReturnKind::Type => api.return_type.rust_type_name().ok_or_else(|| {
            build_error(format!(
                "接口 {}::{} 缺少 Rust 返回类型",
                api.namespace, api.name
            ))
        }),
    }
}

fn kind_name(kind: ApiParameterKind) -> &'static str {
    match kind {
        ApiParameterKind::Body => "Body",
        ApiParameterKind::Query => "Query",
    }
}

/// 收集接口参数与返回值引用的 Rust 类型名。
pub(crate) fn type_names(apis: &[ApiMetadata]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for api in apis {
        for parameter in api.parameters {
            collect_type_names(parameter.rust_type_name(), &mut names);
        }
        if api.return_type.kind == ApiReturnKind::Type
            && let Some(name) = api.return_type.rust_type_name()
        {
            collect_type_names(name, &mut names);
        }
    }
    names
}

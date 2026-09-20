use std::io;
use crate::ApiReturnType;

pub type TypescriptResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// TypeScript 声明与运行时模块的内存内容。
pub struct TypescriptBuildResult {
    pub js: String,
    pub dts: String,
}

/// 构建一个 TypeScript ESM 模块。
pub trait TypescriptBuilder {
    fn build(&self) -> TypescriptResult<TypescriptBuildResult>;
}

pub(crate) fn build_error(message: impl Into<String>) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(io::Error::other(message.into()))
}

pub(crate) fn is_typescript_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    matches!(characters.next(), Some(value) if value.is_ascii_alphabetic() || value == '_')
        && characters.all(|value| value.is_ascii_alphanumeric() || value == '_')
}

pub(crate) fn type_short_name(name: &str) -> String {
    name.rsplit("::").next().unwrap_or(name).to_string()
}

pub(crate) fn typescript_property(name: &str) -> String {
    if is_typescript_identifier(name) {
        name.to_string()
    } else {
        serde_json::to_string(name).unwrap_or_else(|_| format!("\"{name}\""))
    }
}

pub(crate) fn javascript_property(name: &str) -> String {
    typescript_property(name)
}

pub(crate) fn json_string(value: &str) -> TypescriptResult<String> {
    serde_json::to_string(value).map_err(|error| Box::new(error) as _)
}

pub fn return_type_name(return_type: ApiReturnType) -> &'static str {
    match return_type {
        ApiReturnType::Void => "void",
        ApiReturnType::Blob => "Blob",
        ApiReturnType::Type(name) => name,
    }
}
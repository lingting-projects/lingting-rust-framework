use crate::utils::{shouty_snake_case, snake_case};
use crate::{ApiMetadata, ApiParameterKind, ApiParameterMetadata, ApiReturnKind};

use super::api_definition::return_type_name;
use super::common::{RustResult, raw_ident, string_literal};

/// 生成 `ApiCallError` 与 `ApiClient` trait（含每个接口的默认实现）。
pub(crate) fn declaration(trait_name: &str, apis: &[ApiMetadata]) -> RustResult<String> {
    let mut methods = Vec::new();
    for api in apis {
        methods.push(method(api)?);
    }
    Ok(format!(
        "{ERROR}\n\n/// {trait_name}\npub trait {trait_name} {{\n    type Error: std::error::Error + Send + Sync + 'static;\n\n{SEND}\n\n{}\n}}",
        methods.join("\n\n"),
    ))
}

const ERROR: &str = r#"/// 接口调用错误。
#[derive(Debug)]
pub enum ApiCallError<E> {
    /// 传输层错误。
    Transport(E),
    /// 请求或响应 JSON 序列化失败。
    Format(serde_json::Error),
}"#;

const SEND: &str = r#"    /// 发送请求：`body` 与 `query` 为 JSON 文本，返回响应体。
    fn send(
        &self,
        definition: &ApiDefinition,
        body: Option<Vec<u8>>,
        query: Option<Vec<u8>>,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, Self::Error>> + Send;"#;

fn method(api: &ApiMetadata) -> RustResult<String> {
    let name = snake_case(api.name);
    let constant = shouty_snake_case(api.name);
    let return_type = return_type_name(api)?;
    let parameters = api
        .parameters
        .iter()
        .map(|parameter| {
            format!(
                "        {}: {},",
                raw_ident(parameter.name),
                parameter.rust_type_name()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let parameters = if parameters.is_empty() {
        String::new()
    } else {
        format!("{parameters}\n")
    };
    let mut setup = Vec::new();
    let body = request_argument("body", &arguments(api, ApiParameterKind::Body), &mut setup);
    let query = request_argument(
        "query",
        &arguments(api, ApiParameterKind::Query),
        &mut setup,
    );
    let setup = setup
        .into_iter()
        .map(|line| format!("            {line}\n"))
        .collect::<String>();
    let send = format!(
        "self.send(&{constant}, {body}, {query})\n                .await\n                .map_err(ApiCallError::Transport)"
    );
    let tail = match api.return_type.kind {
        ApiReturnKind::Void => format!("            {send}?;\n            Ok(())"),
        ApiReturnKind::Blob => format!("            {send}"),
        ApiReturnKind::Type => format!(
            "            let response = {send}?;\n            serde_json::from_slice(&response).map_err(ApiCallError::Format)"
        ),
    };
    Ok(format!(
        "    /// {} {}\n    fn {name}(\n        &self,\n{parameters}    ) -> impl std::future::Future<Output = Result<{return_type}, ApiCallError<Self::Error>>> + Send\n    where\n        Self: Sync,\n    {{\n        async move {{\n{setup}{tail}\n        }}\n    }}",
        api.method, api.path,
    ))
}

fn arguments(api: &ApiMetadata, kind: ApiParameterKind) -> Vec<&ApiParameterMetadata> {
    api.parameters
        .iter()
        .filter(|parameter| parameter.kind == kind)
        .collect()
}

fn request_argument(
    name: &str,
    parameters: &[&ApiParameterMetadata],
    setup: &mut Vec<String>,
) -> String {
    if parameters.is_empty() {
        return "None".to_string();
    }
    setup.push(format!("let {name} = {};", request_value(parameters)));
    name.to_string()
}

fn request_value(parameters: &[&ApiParameterMetadata]) -> String {
    if let [parameter] = parameters {
        return format!(
            "Some(serde_json::to_vec(&{}).map_err(ApiCallError::Format)?)",
            raw_ident(parameter.name)
        );
    }
    let entries = parameters
        .iter()
        .map(|parameter| {
            format!(
                "{}: {}",
                string_literal(parameter.name),
                raw_ident(parameter.name)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Some(serde_json::to_vec(&serde_json::json!({{ {entries} }})).map_err(ApiCallError::Format)?)"
    )
}

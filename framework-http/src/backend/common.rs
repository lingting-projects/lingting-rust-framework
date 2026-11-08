//! 两个后端共用的请求流程。

use anyhow::Result;

use crate::method::HttpMethod;
use crate::request::HttpRequest;
use crate::response::HttpResponse;
use crate::stream::HttpStreamResponse;

use super::api::Backend;

/// 发送一次请求，响应体整体读入内存。
pub(crate) async fn send<B: Backend>(
    client: &B::Client,
    method: HttpMethod,
    request: HttpRequest,
) -> Result<HttpResponse> {
    let response = B::send(B::build(client, method, &request)).await?;
    let status = B::status(&response);
    let headers = B::headers(&response);
    let body = B::body(response).await?;

    Ok(HttpResponse::new(body, headers, status))
}

/// 取得流式响应，响应体不做整体收集，由调用方逐块消费。
pub(crate) async fn stream<B: Backend>(
    client: &B::Client,
    method: HttpMethod,
    request: HttpRequest,
) -> Result<HttpStreamResponse> {
    let response = B::send(B::build(client, method, &request)).await?;
    let status = B::status(&response);
    let headers = B::headers(&response);

    Ok(HttpStreamResponse::new(
        status,
        headers,
        B::body_stream(response),
    ))
}

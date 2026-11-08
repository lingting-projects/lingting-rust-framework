//! 后端实现：编译期在 wreq 与 reqwest 之间二选一。
//!
//! 目录分工：
//!
//! - `api`：后端必须实现的能力契约；
//! - `common`：基于契约的通用请求流程，两个后端共用一份；
//! - `utils`：两个后端共用的类型转换，请求头、方法与响应头只写一份；
//! - `wreq` / `reqwest`：各后端专属的客户端构建与收发实现。
//!
//! 启用 `wreq` 时使用 wreq；否则使用 reqwest，此时还需启用 `reqwest` feature。

mod api;
mod common;
#[cfg(not(feature = "wreq"))]
mod reqwest;
mod utils;
#[cfg(feature = "wreq")]
mod wreq;

use anyhow::Result;

use crate::method::HttpMethod;
use crate::request::HttpRequest;
use crate::response::HttpResponse;
use crate::stream::HttpStreamResponse;

pub(crate) use api::{Backend, ClientOptions};
#[cfg(not(feature = "wreq"))]
pub(crate) use reqwest::Reqwest as Current;
#[cfg(feature = "wreq")]
pub(crate) use wreq::Wreq as Current;

/// 当前后端使用的客户端。
pub(crate) type Client = <Current as Backend>::Client;

/// 按给定参数构建客户端。
pub(crate) fn create(options: &ClientOptions) -> Result<Client> {
    Current::create(options)
}

/// 发送一次请求，响应体整体读入内存。
pub(crate) async fn send(
    client: &Client,
    method: HttpMethod,
    request: HttpRequest,
) -> Result<HttpResponse> {
    common::send::<Current>(client, method, request).await
}

/// 取得流式响应，响应体不做整体收集。
pub(crate) async fn stream(
    client: &Client,
    method: HttpMethod,
    request: HttpRequest,
) -> Result<HttpStreamResponse> {
    common::stream::<Current>(client, method, request).await
}

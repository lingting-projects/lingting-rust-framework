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

use std::sync::LazyLock;

use anyhow::Result;

use crate::method::HttpMethod;
use crate::request::HttpRequest;
use crate::response::HttpResponse;
use crate::stream::HttpStreamResponse;

pub(crate) use api::Backend;
#[cfg(not(feature = "wreq"))]
pub(crate) use reqwest::Reqwest as Current;
#[cfg(feature = "wreq")]
pub(crate) use wreq::Wreq as Current;

/// 当前后端使用的客户端。
pub(crate) type Client = <Current as Backend>::Client;

/// 默认客户端，不读取任何代理配置。
static DEFAULT: LazyLock<Client> = LazyLock::new(create_default);

/// 共享的默认客户端，复用同一份连接池。
pub(crate) fn shared() -> Client {
    DEFAULT.clone()
}

/// 构建客户端。
///
/// `proxy` 非空时所有请求都走该代理；为空时由 `system_proxy` 决定是否按环境变量与
/// 系统代理设置选择，为假则完全忽略代理。
pub(crate) fn create(proxy: Option<&str>, system_proxy: bool) -> Result<Client> {
    Current::create(proxy, system_proxy)
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

/// 构建默认客户端，失败只可能是环境异常，无需交给调用方处理。
fn create_default() -> Client {
    create(None, false).expect("初始化默认 HTTP 客户端失败")
}

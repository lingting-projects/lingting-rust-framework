//! 后端契约。
//!
//! wreq 与 reqwest 的 API 形状一致但类型不同，这里只抽象出差异点：构建客户端、写入请求、
//! 发送请求、读取状态码与响应头、读取响应体。请求与响应的通用流程由
//! `backend::common` 实现一次，各后端只需给出自己的请求与响应类型。

use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;
use framework_core::MultiStringValue;
use futures_util::Stream;

use crate::common::{DEFAULT_REDIRECT_LIMIT, DEFAULT_TIMEOUT};
use crate::method::HttpMethod;
use crate::request::HttpRequest;

/// 构建后端客户端的参数。
#[derive(Clone, Debug)]
pub(crate) struct ClientOptions {
    /// 显式代理，非空时所有请求都走它
    pub proxy: Option<String>,
    /// 未指定显式代理时，是否按环境变量与系统代理设置选择代理
    pub system_proxy: bool,
    /// 请求超时，`None` 表示不限制
    pub timeout: Option<Duration>,
    /// 最大重定向次数，小于 0 表示不限制，等于 0 表示不跟随
    pub redirect_limit: i64,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            proxy: None,
            system_proxy: false,
            timeout: Some(DEFAULT_TIMEOUT),
            redirect_limit: DEFAULT_REDIRECT_LIMIT,
        }
    }
}

/// 一个 HTTP 后端需要提供的能力。
pub(crate) trait Backend {
    /// 后端客户端
    type Client: Clone + Send + Sync + 'static;
    /// 待发送的请求
    type Request;
    /// 收到的响应
    type Response;

    /// 按给定参数构建客户端。
    fn create(options: &ClientOptions) -> Result<Self::Client>;

    /// 写入请求头与请求体，构建待发送的请求。
    fn build(client: &Self::Client, method: HttpMethod, request: &HttpRequest) -> Self::Request;

    /// 发送请求。
    async fn send(request: Self::Request) -> Result<Self::Response>;

    /// 响应状态码。
    fn status(response: &Self::Response) -> u16;

    /// 响应头。
    fn headers(response: &Self::Response) -> MultiStringValue;

    /// 响应体整体读入内存。
    async fn body(response: Self::Response) -> Result<Bytes>;

    /// 响应体转成逐块到达的流。
    fn body_stream(response: Self::Response) -> impl Stream<Item = Result<Bytes>> + Send + 'static;
}

//! 后端契约。
//!
//! wreq 与 reqwest 的 API 形状一致但类型不同，这里只抽象出差异点：构建客户端、写入请求、
//! 发送请求、读取状态码与响应头、读取响应体。请求与响应的通用流程由
//! `backend::common` 实现一次，各后端只需给出自己的请求与响应类型。

use anyhow::Result;
use bytes::Bytes;
use framework_core::MultiStringValue;
use futures_util::Stream;

use crate::method::HttpMethod;
use crate::request::HttpRequest;

/// 一个 HTTP 后端需要提供的能力。
pub(crate) trait Backend {
    /// 后端客户端
    type Client: Clone + Send + Sync + 'static;
    /// 待发送的请求
    type Request;
    /// 收到的响应
    type Response;

    /// 构建客户端。
    ///
    /// `proxy` 非空时所有请求都走该代理；为空时由 `system_proxy` 决定是否按环境变量与
    /// 系统代理设置选择，为假则完全忽略代理。
    fn create(proxy: Option<&str>, system_proxy: bool) -> Result<Self::Client>;

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

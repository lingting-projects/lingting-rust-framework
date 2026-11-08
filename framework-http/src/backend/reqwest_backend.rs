use std::pin::Pin;
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;
use framework_core::MultiStringValue;
use futures_util::{Stream, StreamExt as _};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;

use crate::header;
use crate::method::HttpMethod;
use crate::request::HttpRequest;
use crate::response::HttpResponse;

/// 请求超时时间。
const TIMEOUT: Duration = Duration::from_secs(60);

/// 跟随的最大重定向次数。
const REDIRECT_LIMIT: usize = 10;

pub(crate) type Client = reqwest::Client;

pub(crate) type Method = reqwest::Method;

/// 默认客户端，不读取任何代理配置。
static DEFAULT: LazyLock<Client> =
    LazyLock::new(|| create(None, false).expect("初始化默认 HTTP 客户端失败"));

/// 共享的默认客户端，复用同一份连接池。
pub(crate) fn shared() -> Client {
    DEFAULT.clone()
}

/// 构建客户端。
///
/// `proxy` 非空时所有请求都走该代理；为空时由 `system_proxy` 决定是否按环境变量与
/// 系统代理设置选择，为假则完全忽略代理。
pub(crate) fn create(proxy: Option<&str>, system_proxy: bool) -> Result<Client> {
    let builder = Client::builder()
        .timeout(TIMEOUT)
        .redirect(Policy::limited(REDIRECT_LIMIT))
        .gzip(true)
        .brotli(true)
        .zstd(true)
        .deflate(true);

    let builder = match proxy {
        Some(proxy) => builder.proxy(reqwest::Proxy::all(proxy)?),
        None if system_proxy => builder,
        None => builder.no_proxy(),
    };

    builder.build().map_err(Into::into)
}

/// 发送一次请求，响应体整体读入内存。
pub(crate) async fn send(
    client: &Client,
    method: HttpMethod,
    request: HttpRequest,
) -> Result<HttpResponse> {
    let response = builder(client, method, &request).send().await?;

    let status = response.status().as_u16();
    let headers = header::from_response(
        response
            .headers()
            .iter()
            .map(|(name, value)| (name.as_str(), String::from_utf8_lossy(value.as_bytes()))),
    );
    let body = response.bytes().await?;

    Ok(HttpResponse::new(body, headers, status))
}

/// 以 GET 取得流式响应，响应体不做整体收集，由调用方逐块消费。
pub(crate) async fn stream(client: &Client, request: HttpRequest) -> Result<FetchStream> {
    let response = builder(client, HttpMethod::Get, &request).send().await?;

    let status = response.status().as_u16();
    let headers = header::from_response(
        response
            .headers()
            .iter()
            .map(|(name, value)| (name.as_str(), String::from_utf8_lossy(value.as_bytes()))),
    );

    Ok(FetchStream {
        status,
        headers,
        body: Box::pin(response.bytes_stream()),
    })
}

/// 流式响应：状态码、响应头与逐块到达的响应体。
pub(crate) struct FetchStream {
    status: u16,
    headers: MultiStringValue,
    body: Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send>>,
}

impl FetchStream {
    /// 是否请求成功。
    pub(crate) fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// 状态码。
    pub(crate) fn status(&self) -> u16 {
        self.status
    }

    /// 读取一个响应头，返回第一个值。
    pub(crate) fn header(&self, name: &str) -> Option<&String> {
        self.headers.get_first(name)
    }

    /// 读取下一块响应体，结束或出错时返回 `None`。
    pub(crate) async fn next(&mut self) -> Option<Result<Bytes>> {
        let chunk = self.body.next().await?;
        Some(chunk.map_err(Into::into))
    }
}

/// 写入请求头与请求体，返回待发送的请求。
fn builder(client: &Client, method: HttpMethod, request: &HttpRequest) -> reqwest::RequestBuilder {
    let builder = client
        .request(method_of(method), request.url.as_str())
        .headers(headers(&request.headers));

    match &request.body {
        Some(body) => builder.body(body.clone()),
        None => builder,
    }
}

/// [`HttpMethod`] 转后端方法。
fn method_of(method: HttpMethod) -> Method {
    Method::from_bytes(method.as_str().as_bytes()).expect("内置的 HTTP 方法名均为合法方法")
}

/// [`MultiStringValue`] 转请求头，同名请求头保留多条。
fn headers(source: &MultiStringValue) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in header::pairs(source) {
        if let (Ok(name), Ok(value)) = (name.parse::<HeaderName>(), value.parse::<HeaderValue>()) {
            headers.append(name, value);
        }
    }
    headers
}

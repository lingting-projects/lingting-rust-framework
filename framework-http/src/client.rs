use std::sync::LazyLock;

use anyhow::Result;
use bytes::Bytes;

use crate::backend;
use crate::builder::HttpClientBuilder;
use crate::method::HttpMethod;
use crate::request::HttpRequest;
use crate::response::HttpResponse;
use crate::stream::HttpStreamResponse;

/// HTTP 客户端。
///
/// 客户端内部持有连接池，克隆时共享同一份连接：[`HttpClient::new`] 拿到的是进程级共享的
/// 默认客户端，其余构造方式都会新建客户端。
#[derive(Clone)]
pub struct HttpClient {
    client: backend::Client,
}

/// 默认客户端，由默认配置构建一次，多次 [`HttpClient::new`] 共享同一份连接池。
static DEFAULT: LazyLock<HttpClient> = LazyLock::new(|| {
    HttpClientBuilder::default()
        .build()
        .expect("初始化默认 HTTP 客户端失败")
});

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient {
    /// 创建客户端，使用默认配置且不使用任何代理：忽略环境变量与系统代理设置。
    ///
    /// 默认客户端在进程内只创建一次，多次调用共享同一份连接池。
    pub fn new() -> Self {
        DEFAULT.clone()
    }

    /// 客户端构建器，用于自定义超时、重定向次数与代理。
    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder::default()
    }

    /// 在默认配置基础上用闭包自定义，内部走 [`HttpClient::builder`]。
    pub fn with(configure: impl FnOnce(HttpClientBuilder) -> HttpClientBuilder) -> Result<Self> {
        configure(Self::builder()).build()
    }

    /// 创建客户端，所有请求都走 `proxy`，其余使用默认配置。
    ///
    /// `proxy` 为代理地址，如 `http://127.0.0.1:7890`、`socks5://127.0.0.1:1080`；
    /// 地址非法时返回错误。
    pub fn with_proxy(proxy: impl AsRef<str>) -> Result<Self> {
        Self::with(|builder| builder.proxy(proxy.as_ref()))
    }

    /// 创建客户端，按环境变量（`HTTP_PROXY`、`HTTPS_PROXY`、`ALL_PROXY`、`NO_PROXY`）
    /// 与操作系统代理设置选择代理，都没有时不走代理，其余使用默认配置。
    pub fn with_system_proxy() -> Result<Self> {
        Self::with(HttpClientBuilder::system_proxy)
    }

    /// 用指定方法发送请求，是本 crate 的原始入口。
    ///
    /// 其余方法都只是把 [`HttpMethod`] 固定下来，请求内容仍由 [`HttpRequest`] 完整描述。
    pub async fn request(&self, method: HttpMethod, request: HttpRequest) -> Result<HttpResponse> {
        backend::send(&self.client, method, request).await
    }

    /// 发送 GET 请求。
    pub async fn get(&self, request: HttpRequest) -> Result<HttpResponse> {
        self.request(HttpMethod::Get, request).await
    }

    /// 发送 POST 请求。
    pub async fn post(&self, request: HttpRequest) -> Result<HttpResponse> {
        self.request(HttpMethod::Post, request).await
    }

    /// 发送 PUT 请求。
    pub async fn put(&self, request: HttpRequest) -> Result<HttpResponse> {
        self.request(HttpMethod::Put, request).await
    }

    /// 发送 PATCH 请求。
    pub async fn patch(&self, request: HttpRequest) -> Result<HttpResponse> {
        self.request(HttpMethod::Patch, request).await
    }

    /// 发送 DELETE 请求。
    pub async fn delete(&self, request: HttpRequest) -> Result<HttpResponse> {
        self.request(HttpMethod::Delete, request).await
    }

    /// 发送 HEAD 请求，只取响应头。
    pub async fn head(&self, request: HttpRequest) -> Result<HttpResponse> {
        self.request(HttpMethod::Head, request).await
    }

    /// 发送 OPTIONS 请求。
    pub async fn options(&self, request: HttpRequest) -> Result<HttpResponse> {
        self.request(HttpMethod::Options, request).await
    }

    /// 快捷方式：以 GET 请求获取远程内容。
    pub async fn fetch_get(&self, url: &str) -> Result<HttpResponse> {
        self.get(HttpRequest::new(url)).await
    }

    /// 快捷方式：以 `body` 为请求体发送 POST 请求。
    pub async fn fetch_post(&self, url: &str, body: impl Into<Bytes>) -> Result<HttpResponse> {
        self.post(HttpRequest::new(url).with_body(body)).await
    }

    /// 原始流式入口：以 GET 请求取得流式响应，响应体逐块返回。
    ///
    /// 一次性的请求用 [`HttpClient::get`] 及其快捷方法，响应体整体读入内存；需要边读边处理
    /// （大文件、事件流）时用本方法。
    pub async fn stream(&self, request: HttpRequest) -> Result<HttpStreamResponse> {
        backend::stream(&self.client, HttpMethod::Get, request).await
    }

    /// 快捷方式：以 GET 请求取得流式响应。
    pub async fn stream_get(&self, url: &str) -> Result<HttpStreamResponse> {
        self.stream(HttpRequest::new(url)).await
    }

    /// 快捷方式：以 `body` 为请求体发送 POST 请求并取得流式响应。
    pub async fn stream_post(
        &self,
        url: &str,
        body: impl Into<Bytes>,
    ) -> Result<HttpStreamResponse> {
        let request = HttpRequest::new(url).with_body(body);
        backend::stream(&self.client, HttpMethod::Post, request).await
    }

    /// 后端客户端，供本 crate 的下载实现复用同一份连接池。
    pub(crate) fn backend(&self) -> &backend::Client {
        &self.client
    }

    /// 用已构建好的后端客户端创建，供 [`HttpClientBuilder`] 使用。
    pub(crate) fn with_backend(client: backend::Client) -> Self {
        Self { client }
    }
}

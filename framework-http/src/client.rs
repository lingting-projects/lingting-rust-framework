use anyhow::Result;
use bytes::Bytes;

use crate::backend;
use crate::method::HttpMethod;
use crate::request::HttpRequest;
use crate::response::HttpResponse;

/// HTTP 客户端。
///
/// 客户端内部持有连接池，克隆时共享同一份连接：[`HttpClient::new`] 拿到的是进程级共享的
/// 默认客户端，[`HttpClient::with_proxy`] 与 [`HttpClient::with_system_proxy`] 会各自新建。
#[derive(Clone)]
pub struct HttpClient {
    client: backend::Client,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient {
    /// 创建客户端，完全不使用代理：忽略环境变量与系统代理设置。
    pub fn new() -> Self {
        Self {
            client: backend::shared(),
        }
    }

    /// 创建客户端，所有请求都走 `proxy`。
    ///
    /// `proxy` 为代理地址，如 `http://127.0.0.1:7890`、`socks5://127.0.0.1:1080`；
    /// 地址非法时返回错误。
    pub fn with_proxy(proxy: impl AsRef<str>) -> Result<Self> {
        Ok(Self {
            client: backend::create(Some(proxy.as_ref()), false)?,
        })
    }

    /// 创建客户端，按环境变量（`HTTP_PROXY`、`HTTPS_PROXY`、`ALL_PROXY`、`NO_PROXY`）
    /// 与操作系统代理设置选择代理，都没有时不走代理。
    pub fn with_system_proxy() -> Result<Self> {
        Ok(Self {
            client: backend::create(None, true)?,
        })
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
    pub async fn fetch(&self, url: &str) -> Result<HttpResponse> {
        self.get(HttpRequest::new(url)).await
    }

    /// 快捷方式：以 `body` 为请求体发送 POST 请求。
    pub async fn fetch_body(&self, url: &str, body: impl Into<Bytes>) -> Result<HttpResponse> {
        self.post(HttpRequest::new(url).with_body(body)).await
    }

    /// 后端客户端，供本 crate 的下载实现复用同一份连接池。
    pub(crate) fn backend(&self) -> &backend::Client {
        &self.client
    }
}

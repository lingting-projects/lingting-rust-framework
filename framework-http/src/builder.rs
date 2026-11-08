//! HTTP 客户端构建器。

use std::time::Duration;

use anyhow::Result;

use crate::backend::{self, ClientOptions};
use crate::client::HttpClient;

/// HTTP 客户端构建器。
///
/// 默认值与 [`HttpClient::new`] 一致：请求超时 60 秒、最多跟随 10 次重定向、不使用任何代理。
/// 需要自定义时用 [`HttpClient::builder`]。
#[derive(Clone, Debug, Default)]
pub struct HttpClientBuilder {
    options: ClientOptions,
}

impl HttpClientBuilder {
    /// 原始值覆盖请求超时，`None` 表示不限制超时。
    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.options.timeout = timeout;
        self
    }

    /// 原始值覆盖最大重定向次数：小于 0 表示不限制，等于 0 表示不跟随。
    pub fn redirect_limit(mut self, limit: i64) -> Self {
        self.options.redirect_limit = limit;
        self
    }

    /// 快捷覆盖：按秒设置请求超时。
    pub fn timeout_secs(self, secs: u64) -> Self {
        self.timeout(Some(Duration::from_secs(secs)))
    }

    /// 快捷覆盖：不限制请求超时。
    pub fn no_timeout(self) -> Self {
        self.timeout(None)
    }

    /// 快捷覆盖：不限制重定向次数。
    pub fn unlimited_redirect(self) -> Self {
        self.redirect_limit(-1)
    }

    /// 快捷覆盖：不跟随重定向，直接返回 3xx 响应。
    pub fn no_redirect(self) -> Self {
        self.redirect_limit(0)
    }

    /// 所有请求都走 `proxy`，如 `http://127.0.0.1:7890`、`socks5://127.0.0.1:1080`。
    ///
    /// 与 [`system_proxy`](Self::system_proxy) 同时设置时以显式代理为准。
    pub fn proxy(mut self, proxy: impl Into<String>) -> Self {
        self.options.proxy = Some(proxy.into());
        self
    }

    /// 按环境变量（`HTTP_PROXY`、`HTTPS_PROXY`、`ALL_PROXY`、`NO_PROXY`）与操作系统代理设置
    /// 选择代理，都没有时不走代理。
    pub fn system_proxy(mut self) -> Self {
        self.options.system_proxy = true;
        self
    }

    /// 构建客户端，代理地址非法时返回错误。
    pub fn build(self) -> Result<HttpClient> {
        Ok(HttpClient::with_backend(backend::create(&self.options)?))
    }
}

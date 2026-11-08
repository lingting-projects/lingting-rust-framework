use std::pin::Pin;

use anyhow::Result;
use bytes::Bytes;
use framework_core::MultiStringValue;
use futures_util::{Stream, StreamExt as _};

/// 流式响应：状态码、响应头与逐块到达的响应体。
///
/// 由 [`HttpClient::stream`](crate::HttpClient::stream) 及其快捷方法返回。响应体不做整体收集，
/// 调用方用 [`next`](Self::next) 逐块消费，因此大文件不会一次性进内存。
pub struct HttpStreamResponse {
    /// 状态码
    pub status: u16,
    /// 响应头
    pub headers: MultiStringValue,
    /// 逐块到达的响应体
    body: Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>,
}

impl HttpStreamResponse {
    /// 由状态码、响应头与响应体流构建。
    pub(crate) fn new(
        status: u16,
        headers: MultiStringValue,
        body: impl Stream<Item = Result<Bytes>> + Send + 'static,
    ) -> Self {
        Self {
            status,
            headers,
            body: Box::pin(body),
        }
    }

    /// 是否请求成功。
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// 读取响应头，返回第一个值。
    pub fn header(&self, name: &str) -> Option<&String> {
        self.headers.get_first(name)
    }

    /// `content-type` 响应头。
    pub fn content_type(&self) -> Option<&String> {
        self.headers.content_type()
    }

    /// 不含参数的 `content-type`。
    pub fn mime_type(&self) -> Option<String> {
        self.headers.mime_type()
    }

    /// `content-length` 响应头。
    pub fn content_length(&self) -> Option<u64> {
        self.header("content-length")?.trim().parse().ok()
    }

    /// 读取下一块响应体，流结束或出错时返回 `None`。
    pub async fn next(&mut self) -> Option<Result<Bytes>> {
        self.body.next().await
    }
}

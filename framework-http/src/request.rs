use std::collections::HashMap;

use bytes::Bytes;
use framework_core::MultiStringValue;

/// User-Agent 请求头名称。
pub const HEADER_UA: &str = "user-agent";

/// 请求信息。
///
/// 必填字段为 [`url`](Self::url)，其余按需设置。
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// 请求地址
    pub url: String,
    /// 请求体，为 `None` 时不携带请求体
    pub body: Option<Bytes>,
    /// 请求头
    pub headers: MultiStringValue,
}

impl HttpRequest {
    /// 由请求地址构建。
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            body: None,
            headers: MultiStringValue::create(true, Default::default()),
        }
    }

    /// 设置请求体。
    pub fn with_body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// 写入一个请求头，同名请求头会被覆盖。
    pub fn with(mut self, name: impl ToString, value: impl ToString) -> Self {
        self.headers.set(name, value);
        self
    }

    /// 写入多个请求头，同名请求头保留多条。
    pub fn with_headers(mut self, headers: &MultiStringValue) -> Self {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        headers.for_each(|name, values| {
            map.insert(name.clone(), values.clone());
        });
        self.headers.move_from(map);
        self
    }

    /// 写入 User-Agent 请求头。
    pub fn with_ua(self, ua: impl ToString) -> Self {
        self.with(HEADER_UA, ua)
    }

    /// 写入 `content-type` 请求头。
    pub fn with_content_type(self, value: impl ToString) -> Self {
        self.with("content-type", value)
    }

    /// 写入 `accept` 请求头。
    pub fn with_accept(self, value: impl ToString) -> Self {
        self.with("accept", value)
    }

    /// 写入 `authorization` 请求头。
    pub fn with_authorization(self, value: impl ToString) -> Self {
        self.with("authorization", value)
    }

    /// 是否已设置 User-Agent 请求头。
    pub fn has_ua(&self) -> bool {
        self.headers.get_first(HEADER_UA).is_some()
    }
}

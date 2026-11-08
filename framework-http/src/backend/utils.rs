//! 两个后端共用的基础类型转换。
//!
//! wreq 与 reqwest 都直接重导出 `http` 的 `Method` 与 `HeaderMap`，转换因此只写一份。

use framework_core::MultiStringValue;
use http::{HeaderMap, HeaderName, HeaderValue, Method};

use crate::method::HttpMethod;

/// [`MultiStringValue`] 转请求头，同名请求头保留多条。
pub(crate) fn headers(source: &MultiStringValue) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in crate::utils::pairs(source) {
        if let (Ok(name), Ok(value)) = (name.parse::<HeaderName>(), value.parse::<HeaderValue>()) {
            headers.append(name, value);
        }
    }

    headers
}

/// [`HttpMethod`] 转后端方法。
pub(crate) fn method(method: HttpMethod) -> Method {
    Method::from_bytes(method.as_str().as_bytes()).expect("内置的 HTTP 方法名均为合法方法")
}

/// 响应头转 [`MultiStringValue`]，名称统一小写。
pub(crate) fn response_headers(headers: &HeaderMap) -> MultiStringValue {
    crate::utils::from_response(
        headers
            .iter()
            .map(|(name, value)| (name.as_str(), String::from_utf8_lossy(value.as_bytes()))),
    )
}

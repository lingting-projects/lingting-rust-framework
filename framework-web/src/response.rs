use crate::{WebError, WebErrorKind, WebRequest};
use anyhow::{Error, Result};
use bytes::Bytes;
use framework_core::MultiStringValue;
use framework_core::types::R;
use futures_util::stream::BoxStream;
use serde::Serialize;
use serde_json::json;

const INTERNAL_ERROR_BODY: &[u8] = br#"{"code":500,"message":"Server Error"}"#;

/// 响应体：一次性字节内容或持续输出的流。
pub enum WebBody {
    /// 一次性返回的字节内容。
    Bytes(Bytes),
    /// 流式返回的内容，用于 SSE 等场景。
    Stream(BoxStream<'static, Result<Bytes, std::io::Error>>),
}

impl WebBody {
    /// 是否为流式响应体。
    pub fn is_stream(&self) -> bool {
        matches!(self, Self::Stream(_))
    }
}

impl Default for WebBody {
    fn default() -> Self {
        Self::Bytes(Bytes::new())
    }
}

impl From<Bytes> for WebBody {
    fn from(value: Bytes) -> Self {
        Self::Bytes(value)
    }
}

impl From<Vec<u8>> for WebBody {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(Bytes::from(value))
    }
}

impl From<String> for WebBody {
    fn from(value: String) -> Self {
        Self::Bytes(Bytes::from(value))
    }
}

impl From<BoxStream<'static, Result<Bytes, std::io::Error>>> for WebBody {
    fn from(value: BoxStream<'static, Result<Bytes, std::io::Error>>) -> Self {
        Self::Stream(value)
    }
}

pub struct WebResponse {
    pub status: u16,
    pub headers: MultiStringValue,
    pub body: WebBody,
}

impl WebResponse {
    pub fn empty() -> Self {
        Self {
            status: 204,
            headers: MultiStringValue::default(),
            body: WebBody::Bytes(Bytes::new()),
        }
    }

    pub fn from_error(error: Error, request: Option<&WebRequest>) -> Self {
        let error = normalize_error(error);
        WebError::log(&error, request);
        Self::error_body(&error)
    }

    pub fn from_error_request(error: Error, request_id: &str, method: &str, path: &str) -> Self {
        let error = normalize_error(error);
        WebError::log_request(&error, request_id, method, path);
        Self::error_body(&error)
    }

    fn error_body(error: &WebError) -> Self {
        let status = error.status();
        let message = error.public_message();
        Self::json(status, &json!({ "code": status, "message": message }))
    }

    pub fn from_result(result: Result<Self>, request: Option<&WebRequest>) -> Self {
        match result {
            Ok(response) => response,
            Err(error) => Self::from_error(error, request),
        }
    }

    pub fn from_t<T>(value: T, request: Option<&WebRequest>) -> Self
    where
        T: Serialize,
    {
        let r = R::ok(value);
        Self::serialize(&r, request)
    }

    pub fn from_r<T>(value: R<T>, request: Option<&WebRequest>) -> Self
    where
        T: Serialize,
    {
        Self::serialize(&value, request)
    }

    pub fn from_result_t<T>(result: Result<T>, request: Option<&WebRequest>) -> Self
    where
        T: Serialize,
    {
        match result {
            Ok(value) => Self::from_t(value, request),
            Err(error) => Self::from_error(error, request),
        }
    }

    pub fn from_result_r<T>(result: Result<R<T>>, request: Option<&WebRequest>) -> Self
    where
        T: Serialize,
    {
        match result {
            Ok(value) => Self::from_r(value, request),
            Err(error) => Self::from_error(error, request),
        }
    }

    fn serialize<T>(value: &T, request: Option<&WebRequest>) -> Self
    where
        T: Serialize,
    {
        match serde_json::to_vec(value) {
            Ok(body) => Self::json_bytes(200, body),
            Err(error) => Self::from_error(
                Error::from(WebError::return_conversion("返回值 JSON 序列化失败", error)),
                request,
            ),
        }
    }

    fn json<T>(status: u16, value: &T) -> Self
    where
        T: Serialize,
    {
        match serde_json::to_vec(value) {
            Ok(body) => Self::json_bytes(status, body),
            Err(_) => Self::json_bytes(500, INTERNAL_ERROR_BODY.to_vec()),
        }
    }

    fn json_bytes(status: u16, body: Vec<u8>) -> Self {
        let mut headers = MultiStringValue::default();
        headers.set_content_type("application/json; charset=utf-8");
        headers.set_content_length(body.len());
        Self {
            status,
            headers,
            body: WebBody::Bytes(Bytes::from(body)),
        }
    }

    /// 构造流式响应，用于 SSE 等持续输出场景。
    pub fn stream(
        status: u16,
        content_type: &str,
        stream: BoxStream<'static, Result<Bytes, std::io::Error>>,
    ) -> Self {
        let mut headers = MultiStringValue::default();
        headers.set_content_type(content_type);
        Self {
            status,
            headers,
            body: WebBody::Stream(stream),
        }
    }
}

fn normalize_error(error: Error) -> WebError {
    error.downcast::<WebError>().unwrap_or_else(|error| {
        if let Some(message) = error.downcast_ref::<&'static str>() {
            WebError::with_source(WebErrorKind::Message, *message, error)
        } else if let Some(message) = error.downcast_ref::<String>() {
            WebError::with_source(WebErrorKind::Message, message.clone(), error)
        } else {
            WebError::internal("请求处理发生内部错误", error)
        }
    })
}

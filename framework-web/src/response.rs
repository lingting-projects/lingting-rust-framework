use crate::{WebError, WebRequest};
use anyhow::{Error, Result};
use bytes::Bytes;
use framework_core::types::R;
use framework_core::value::MultiStringValue;
use serde::Serialize;
use serde_json::json;

const INTERNAL_ERROR_BODY: &[u8] = br#"{"code":500,"message":"Server Error"}"#;

pub struct WebResponse {
    pub status: u16,
    pub headers: MultiStringValue,
    pub body: Bytes,
}

impl WebResponse {
    pub fn empty() -> Self {
        Self {
            status: 204,
            headers: MultiStringValue::default(),
            body: Bytes::new(),
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

    fn error_body(error: &Error) -> Self {
        let web_error = error.downcast_ref::<WebError>();
        let status = web_error.map_or(500, WebError::status);
        let message = web_error.map_or("服务器内部错误", WebError::public_message);
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
            body: Bytes::from(body),
        }
    }
}

fn normalize_error(error: Error) -> Error {
    if error.is::<WebError>() {
        error
    } else {
        Error::from(WebError::internal("请求处理发生内部错误", error))
    }
}

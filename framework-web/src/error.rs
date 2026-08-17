use crate::WebRequest;
use anyhow::Error;
use framework_core::types::RCodeKind;
use log::{error, info, warn};
use std::fmt::{Display, Formatter};
use std::panic::Location;

#[derive(Debug, PartialEq, Eq)]
pub enum WebErrorKind {
    Parameter,
    ReturnConversion,
    NotFound,
    Unauthorized,
    Forbidden,
    Internal,
    Panic,
}

#[derive(Debug)]
pub struct WebError {
    kind: WebErrorKind,
    message: String,
    source: Option<Error>,
    location: &'static Location<'static>,
}

impl WebError {
    #[track_caller]
    pub fn parameter(message: impl Into<String>, source: impl Display) -> Self {
        Self::with_source(WebErrorKind::Parameter, message, source)
    }

    #[track_caller]
    pub fn return_conversion(message: impl Into<String>, source: impl Display) -> Self {
        Self::with_source(WebErrorKind::ReturnConversion, message, source)
    }

    #[track_caller]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(WebErrorKind::NotFound, message)
    }

    #[track_caller]
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(WebErrorKind::Unauthorized, message)
    }

    #[track_caller]
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(WebErrorKind::Forbidden, message)
    }

    #[track_caller]
    pub fn internal(message: impl Into<String>, source: impl Display) -> Self {
        Self::with_source(WebErrorKind::Internal, message, source)
    }

    #[track_caller]
    pub fn panic(message: impl Into<String>) -> Self {
        Self::new(WebErrorKind::Panic, message)
    }

    pub fn kind(&self) -> &WebErrorKind {
        &self.kind
    }

    pub fn status(&self) -> u16 {
        match self.kind {
            WebErrorKind::Parameter => RCodeKind::Parameter.code() as u16,
            WebErrorKind::Unauthorized => RCodeKind::Unauthorized.code() as u16,
            WebErrorKind::Forbidden => RCodeKind::Forbidden.code() as u16,
            WebErrorKind::NotFound => 404,
            WebErrorKind::ReturnConversion | WebErrorKind::Internal | WebErrorKind::Panic => {
                RCodeKind::Internal.code() as u16
            }
        }
    }

    pub fn public_message(&self) -> &str {
        match self.kind {
            WebErrorKind::Internal | WebErrorKind::ReturnConversion | WebErrorKind::Panic => {
                "服务器内部错误"
            }
            _ => &self.message,
        }
    }

    pub fn log(error: &Error, request: Option<&WebRequest>) {
        let request_id = request.map_or("未知", |item| item.request_id.as_str());
        let method = request.map_or_else(|| "未知".to_string(), |item| item.method.to_string());
        let path = request.map_or("未知", |item| item.path.as_str());
        Self::log_request(error, request_id, &method, path);
    }

    pub fn log_request(error: &Error, request_id: &str, method: &str, path: &str) {
        let web_error = error.downcast_ref::<Self>();
        let status = web_error.map_or(500, Self::status);
        let kind = web_error.map_or("内部错误", |item| item.kind_name());
        let location = web_error.map_or_else(
            || "未知".to_string(),
            |item| {
                format!(
                    "{}:{}:{}",
                    item.location.file(),
                    item.location.line(),
                    item.location.column()
                )
            },
        );
        let request_message = format!("request_id={request_id} method={method} path={path}");
        let chain = error
            .chain()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" -> ");
        let message = format!(
            "Web 请求异常 category={kind} status={status} {request_message} source={location} error_chain={chain} backtrace={}",
            error.backtrace()
        );

        match web_error.map(Self::kind) {
            Some(WebErrorKind::Parameter) => info!("{message}"),
            Some(WebErrorKind::NotFound)
            | Some(WebErrorKind::Unauthorized)
            | Some(WebErrorKind::Forbidden) => warn!("{message}"),
            _ => error!("{message}"),
        }
    }

    #[track_caller]
    fn new(kind: WebErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
            location: Location::caller(),
        }
    }

    #[track_caller]
    fn with_source(kind: WebErrorKind, message: impl Into<String>, source: impl Display) -> Self {
        Self {
            kind,
            message: message.into(),
            source: Some(Error::msg(source.to_string())),
            location: Location::caller(),
        }
    }

    fn kind_name(&self) -> &'static str {
        match self.kind {
            WebErrorKind::Parameter => "参数转换",
            WebErrorKind::ReturnConversion => "返回值转换",
            WebErrorKind::NotFound => "路由不存在",
            WebErrorKind::Unauthorized => "未授权",
            WebErrorKind::Forbidden => "无权限",
            WebErrorKind::Internal => "内部错误",
            WebErrorKind::Panic => "程序崩溃",
        }
    }
}

impl Display for WebError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WebError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|error| error.as_ref())
    }
}

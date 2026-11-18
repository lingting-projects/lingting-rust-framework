use crate::WebRequest;
use anyhow::Error;
use framework_core::types::RCodeKind;
use log::{error, warn};
use std::any::Any;
use std::fmt::{Display, Formatter};
use std::panic::Location;

/// HTTP 状态码合法范围的最小值。
const HTTP_STATUS_MIN: u16 = 100;
/// HTTP 状态码合法范围的最大值。
const HTTP_STATUS_MAX: u16 = 599;

#[derive(Debug, PartialEq, Eq)]
pub enum WebErrorKind {
    Message,
    Parameter,
    ReturnConversion,
    NotFound,
    Unauthorized,
    Forbidden,
    Internal,
    Panic,
    /// 自定义 HTTP 状态码，R 标准结构状态码与 HTTP 状态码一致。
    Status(u16),
    /// 自定义 R 标准结构状态码，HTTP 状态码固定为 200。
    Code(u32),
    /// 同时自定义 HTTP 状态码与 R 标准结构状态码。
    StatusAndCode {
        /// HTTP 状态码。
        status: u16,
        /// R 标准结构状态码。
        code: u32,
    },
}

impl WebErrorKind {
    /// 规范化自定义状态码：非法的 HTTP 状态码回退为 500，R 状态码保持不变。
    #[track_caller]
    fn normalize(self) -> Self {
        match self {
            Self::Status(status) => Self::Status(WebError::valid_http_status(status)),
            Self::StatusAndCode { status, code } => Self::StatusAndCode {
                status: WebError::valid_http_status(status),
                code,
            },
            other => other,
        }
    }
}

#[derive(Debug)]
pub struct WebError {
    kind: WebErrorKind,
    message: String,
    source: Option<Error>,
    location: Option<String>,
}

impl WebError {
    #[track_caller]
    pub fn message(message: impl Into<String>) -> Self {
        Self::new(WebErrorKind::Message, message)
    }

    #[track_caller]
    pub fn parameter(
        message: impl Into<String>,
        source: impl Display + Send + Sync + 'static,
    ) -> Self {
        Self::with_source(WebErrorKind::Parameter, message, source)
    }

    #[track_caller]
    pub fn return_conversion(
        message: impl Into<String>,
        source: impl Display + Send + Sync + 'static,
    ) -> Self {
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
    pub fn internal(
        message: impl Into<String>,
        source: impl Display + Send + Sync + 'static,
    ) -> Self {
        Self::with_source(WebErrorKind::Internal, message, source)
    }

    #[track_caller]
    pub fn panic(message: impl Into<String>) -> Self {
        Self::new(WebErrorKind::Panic, message)
    }

    /// 自定义 HTTP 状态码，R 标准结构状态码与 HTTP 状态码一致。
    #[track_caller]
    pub fn custom_status(status: u16, message: impl Into<String>) -> Self {
        Self::new(WebErrorKind::Status(status), message)
    }

    /// 自定义 R 标准结构状态码，HTTP 状态码固定为 200。
    #[track_caller]
    pub fn custom_code(code: u32, message: impl Into<String>) -> Self {
        Self::new(WebErrorKind::Code(code), message)
    }

    /// 同时自定义 HTTP 状态码与 R 标准结构状态码。
    #[track_caller]
    pub fn custom_status_code(status: u16, code: u32, message: impl Into<String>) -> Self {
        Self::new(WebErrorKind::StatusAndCode { status, code }, message)
    }

    /// 覆盖 HTTP 状态码，R 标准结构状态码保持不变；非法状态码回退为 500。
    #[track_caller]
    pub fn with_status(mut self, status: u16) -> Self {
        let code = self.code();
        self.kind = WebErrorKind::StatusAndCode {
            status: Self::valid_http_status(status),
            code,
        };
        self
    }

    /// 覆盖 R 标准结构状态码，HTTP 状态码保持不变。
    pub fn with_code(mut self, code: u32) -> Self {
        let status = self.status();
        self.kind = WebErrorKind::StatusAndCode { status, code };
        self
    }

    pub fn kind(&self) -> &WebErrorKind {
        &self.kind
    }

    pub fn status(&self) -> u16 {
        match self.kind {
            WebErrorKind::Message => RCodeKind::Internal.code() as u16,
            WebErrorKind::Parameter => RCodeKind::Parameter.code() as u16,
            WebErrorKind::Unauthorized => RCodeKind::Unauthorized.code() as u16,
            WebErrorKind::Forbidden => RCodeKind::Forbidden.code() as u16,
            WebErrorKind::NotFound => 404,
            WebErrorKind::ReturnConversion | WebErrorKind::Internal | WebErrorKind::Panic => {
                RCodeKind::Internal.code() as u16
            }
            WebErrorKind::Status(status) => status,
            WebErrorKind::Code(_) => 200,
            WebErrorKind::StatusAndCode { status, .. } => status,
        }
    }

    pub fn code(&self) -> u32 {
        match self.kind {
            WebErrorKind::Message => RCodeKind::Internal.code(),
            WebErrorKind::Parameter => RCodeKind::Parameter.code(),
            WebErrorKind::Unauthorized => RCodeKind::Unauthorized.code(),
            WebErrorKind::Forbidden => RCodeKind::Forbidden.code(),
            WebErrorKind::NotFound => 404,
            WebErrorKind::ReturnConversion | WebErrorKind::Internal | WebErrorKind::Panic => {
                RCodeKind::Internal.code()
            }
            WebErrorKind::Status(status) => u32::from(status),
            WebErrorKind::Code(code) => code,
            WebErrorKind::StatusAndCode { code, .. } => code,
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

    pub fn log(error: &Self, request: Option<&WebRequest>) {
        let trace_id = request.map_or("未知", |item| item.trace_id.as_str());
        let method = request.map_or_else(|| "未知".to_string(), |item| item.method.to_string());
        let path = request.map_or("未知", |item| item.path.as_str());
        Self::log_request(error, trace_id, &method, path);
    }

    pub fn log_request(error: &Self, trace_id: &str, method: &str, path: &str) {
        let status = error.status();
        let code = error.code();
        let location = error.location.as_deref().unwrap_or("未知");
        let chain = std::iter::successors(Some(error as &dyn std::error::Error), |error| {
            error.source()
        })
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" -> ");
        let backtrace = error
            .source_backtrace()
            .map(ToString::to_string)
            .unwrap_or_else(|| "未知".to_string());

        error!(
            "Web 请求异常 status={status} code={code} \
            trace_id={trace_id} method={method} path={path} \
            source={location} error_chain={chain} backtrace={backtrace}"
        )
    }

    #[track_caller]
    fn new(kind: WebErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind: kind.normalize(),
            message: message.into(),
            source: None,
            location: Some(Self::location(Location::caller())),
        }
    }

    #[track_caller]
    pub fn with_source(
        kind: WebErrorKind,
        message: impl Into<String>,
        source: impl Display + Send + Sync + 'static,
    ) -> Self {
        let source_message = source.to_string();
        let source = Box::new(source) as Box<dyn Any + Send + Sync>;
        let source = match source.downcast::<Error>() {
            Ok(source) => *source,
            Err(source) => match source.downcast::<WebError>() {
                Ok(source) => Error::new(*source),
                Err(_) => Error::msg(source_message),
            },
        };
        #[cfg(debug_assertions)]
        let location = Self::location_from_error(&source);
        #[cfg(not(debug_assertions))]
        let location = None;

        Self {
            kind: kind.normalize(),
            message: message.into(),
            source: Some(source),
            location,
        }
    }

    fn location(location: &Location<'_>) -> String {
        format!(
            "{}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        )
    }

    fn source_backtrace(&self) -> Option<&std::backtrace::Backtrace> {
        self.source.as_ref().map(Error::backtrace)
    }

    fn location_from_error(error: &Error) -> Option<String> {
        error
            .chain()
            .find_map(|error| error.downcast_ref::<Self>())
            .and_then(|error| error.location.clone())
            .or_else(|| Self::backtrace_location(error.backtrace()))
    }

    fn backtrace_location(backtrace: &std::backtrace::Backtrace) -> Option<String> {
        backtrace
            .to_string()
            .lines()
            .map(str::trim)
            .filter_map(|line| line.strip_prefix("at "))
            .find(|location| {
                !location.starts_with("/rustc/")
                    && !location.contains("\\.cargo\\registry\\")
                    && !location.contains("/.cargo/registry/")
            })
            .map(ToOwned::to_owned)
    }
    /// 校验并规范化 HTTP 状态码，非法值回退为 500。
    #[track_caller]
    fn valid_http_status(status: u16) -> u16 {
        if (HTTP_STATUS_MIN..=HTTP_STATUS_MAX).contains(&status) {
            return status;
        }
        warn!(
            "非法 HTTP 状态码 {status}（合法范围 {HTTP_STATUS_MIN}..={HTTP_STATUS_MAX}），\
            已回退为 500，来源 {}",
            Location::caller()
        );
        500
    }
}

pub trait WebErrorExt<T> {
    fn message(self) -> anyhow::Result<T>;
    fn parameter(self, message: impl Into<String>) -> anyhow::Result<T>;
    fn internal(self, message: impl Into<String>) -> anyhow::Result<T>;
}

impl<T, E> WebErrorExt<T> for Result<T, E>
where
    E: Display + Send + Sync + 'static,
{
    #[track_caller]
    fn message(self) -> anyhow::Result<T> {
        self.map_err(|error| {
            WebError::with_source(WebErrorKind::Message, error.to_string(), error).into()
        })
    }

    #[track_caller]
    fn parameter(self, message: impl Into<String>) -> anyhow::Result<T> {
        self.map_err(|error| WebError::parameter(message, error).into())
    }

    #[track_caller]
    fn internal(self, message: impl Into<String>) -> anyhow::Result<T> {
        self.map_err(|error| WebError::internal(message, error).into())
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

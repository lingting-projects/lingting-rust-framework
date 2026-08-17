use bytes::Bytes;
use framework_core::value::MultiStringValue;
use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Eq)]
pub enum WebMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Options,
    Other(String),
}

impl WebMethod {
    pub fn from_name(value: &str) -> Self {
        let value = value.to_ascii_uppercase();
        match value.as_str() {
            "GET" => Self::Get,
            "POST" => Self::Post,
            "PUT" => Self::Put,
            "PATCH" => Self::Patch,
            "DELETE" => Self::Delete,
            "OPTIONS" => Self::Options,
            _ => Self::Other(value),
        }
    }
}

impl Display for WebMethod {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Get => formatter.write_str("GET"),
            Self::Post => formatter.write_str("POST"),
            Self::Put => formatter.write_str("PUT"),
            Self::Patch => formatter.write_str("PATCH"),
            Self::Delete => formatter.write_str("DELETE"),
            Self::Options => formatter.write_str("OPTIONS"),
            Self::Other(value) => formatter.write_str(value),
        }
    }
}

#[derive(Debug)]
pub struct WebRequest {
    pub method: WebMethod,
    pub scheme: String,
    pub authority: String,
    pub path: String,
    pub headers: MultiStringValue,
    pub query: MultiStringValue,
    pub body: Bytes,
    pub client_ip: Option<String>,
    pub request_id: String,
}

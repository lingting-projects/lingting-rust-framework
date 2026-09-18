use crate::{AxumContext, AxumCorsFn, WebRouter};
use anyhow::{Context, Result};
use axum::body::{Body, to_bytes};
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::Response;
use framework_core::{MultiStringValue, next_id};
use framework_web::{WebBody, WebContext, WebMethod, WebRequest, WebResponse, catch_panic};
use log::error;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

/// 请求体读取上限。
const MAX_BODY_SIZE: usize = 16 * 1024 * 1024;

/// 请求分发所需的共享状态。
#[derive(Clone)]
pub(crate) struct AxumState {
    pub(crate) router: Arc<WebRouter>,
    pub(crate) context: AxumContext,
    pub(crate) cors: AxumCorsFn,
}

pub(crate) async fn dispatch(
    State(state): State<AxumState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
) -> Response {
    dispatch_inner(state, peer, request)
        .await
        .unwrap_or_else(|error| {
            to_axum_response(WebResponse::from_error(error, None)).unwrap_or_else(|error| {
                error!("Axum 响应转换失败：{error}");
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap_or_default()
            })
        })
}

async fn dispatch_inner(state: AxumState, peer: SocketAddr, request: Request) -> Result<Response> {
    let request_id = request_id(&request);
    let method = request.method().to_string();
    let uri = request.uri().clone();
    let mut headers = HashMap::<String, Vec<String>>::new();
    for (name, value) in request.headers() {
        headers
            .entry(name.as_str().to_ascii_lowercase())
            .or_default()
            .push(value.to_str().unwrap_or_default().to_string());
    }
    let body = to_bytes(request.into_body(), MAX_BODY_SIZE)
        .await
        .context("读取请求体失败")?;
    let mut query = HashMap::<String, Vec<String>>::new();
    for (name, value) in url::form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes()) {
        query
            .entry(name.into_owned())
            .or_default()
            .push(value.into_owned());
    }
    let web_request = Arc::new(WebRequest {
        method: WebMethod::from_name(&method),
        scheme: uri.scheme_str().unwrap_or("http").into(),
        authority: uri.authority().map_or_else(
            || {
                headers
                    .get("host")
                    .and_then(|values| values.first())
                    .cloned()
                    .unwrap_or_default()
            },
            ToString::to_string,
        ),
        path: uri.path().into(),
        headers: MultiStringValue::create(true, headers),
        query: MultiStringValue::create(false, query),
        body,
        client_ip: Some(peer.ip().to_string()),
        request_id: request_id.clone(),
    });
    let context = WebContext::new(Arc::clone(&web_request));
    let cors = (state.cors)(&context);
    let mut response = if web_request.method == WebMethod::Options {
        WebResponse::empty()
    } else {
        let result = catch_panic(state.router.invoke(context, state.context.clone())).await;
        match result {
            Ok(Ok(response)) => response,
            Ok(Err(error)) | Err(error) => WebResponse::from_error(error, Some(&web_request)),
        }
    };
    cors.apply(&mut response.headers);
    response.headers.set("x-request-id", &request_id);
    to_axum_response(response)
}

fn request_id(request: &Request) -> String {
    request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            next_id()
                .map(|value| value.to_string())
                .unwrap_or_else(|_| "request-id-unavailable".into())
        })
}

fn to_axum_response(response: WebResponse) -> Result<Response> {
    let mut builder = Response::builder().status(response.status);
    for name in response.headers.keys() {
        if let Some(values) = response.headers.get(name) {
            for value in values {
                builder = builder.header(
                    HeaderName::try_from(name.as_str())?,
                    HeaderValue::try_from(value)?,
                );
            }
        }
    }
    let body = match response.body {
        WebBody::Bytes(bytes) => Body::from(bytes),
        WebBody::Stream(stream) => Body::from_stream(stream),
    };
    builder.body(body).map_err(Into::into)
}

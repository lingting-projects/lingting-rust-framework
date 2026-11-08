//! reqwest 后端：未启用 `wreq` 时使用。

use anyhow::Result;
use bytes::Bytes;
use framework_core::MultiStringValue;
use futures_util::{Stream, StreamExt as _};
use reqwest::redirect::Policy;

use crate::method::HttpMethod;
use crate::request::HttpRequest;

use super::api::{Backend, ClientOptions};
use super::utils;

/// reqwest 后端。
pub(crate) struct Reqwest;

impl Backend for Reqwest {
    type Client = reqwest::Client;
    type Request = reqwest::RequestBuilder;
    type Response = reqwest::Response;

    fn create(options: &ClientOptions) -> Result<Self::Client> {
        let builder = Self::Client::builder()
            .gzip(true)
            .brotli(true)
            .zstd(true)
            .deflate(true);

        let builder = match options.timeout {
            Some(timeout) => builder.timeout(timeout),
            None => builder,
        };

        let builder = match utils::redirect(options.redirect_limit) {
            Some(limit) => builder.redirect(Policy::limited(limit)),
            None => builder.redirect(Policy::none()),
        };

        let builder = match options.proxy.as_deref() {
            Some(proxy) => builder.proxy(reqwest::Proxy::all(proxy)?),
            None if options.system_proxy => builder,
            None => builder.no_proxy(),
        };

        builder.build().map_err(Into::into)
    }

    fn build(client: &Self::Client, method: HttpMethod, request: &HttpRequest) -> Self::Request {
        let builder = client
            .request(utils::method(method), request.url.as_str())
            .headers(utils::headers(&request.headers));

        match &request.body {
            Some(body) => builder.body(body.clone()),
            None => builder,
        }
    }

    async fn send(request: Self::Request) -> Result<Self::Response> {
        Ok(request.send().await?)
    }

    fn status(response: &Self::Response) -> u16 {
        response.status().as_u16()
    }

    fn headers(response: &Self::Response) -> MultiStringValue {
        utils::response_headers(response.headers())
    }

    async fn body(response: Self::Response) -> Result<Bytes> {
        Ok(response.bytes().await?)
    }

    fn body_stream(response: Self::Response) -> impl Stream<Item = Result<Bytes>> + Send + 'static {
        response
            .bytes_stream()
            .map(|chunk| chunk.map_err(Into::into))
    }
}

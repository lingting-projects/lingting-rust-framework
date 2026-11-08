//! 统一的 HTTP 请求、下载与重试。
//!
//! 对外提供三件事：
//!
//! - [`HttpClient`]：可配置代理的客户端，既支持 [`HttpMethod`] 指定的原始请求，
//!   也支持按方法固定、只给地址的快捷调用；
//! - [`HttpRequest`] / [`HttpResponse`]：请求与响应模型，请求头与响应头统一使用
//!   [`MultiStringValue`](framework_core::MultiStringValue)；
//! - 下载：[`HttpClient::probe`] 探测远端文件大小与是否支持分段，[`HttpClient::download`]
//!   据此决定分片并行还是单点顺序下载。
//!
//! 后端由 feature 决定，crate 其余部分不感知差异：启用 `reqwest` 时使用 reqwest；启用
//! `wreq` 时使用 wreq 并模拟 Chrome 的 TLS 指纹，两者都启用时使用 wreq。后端 feature 默认
//! 全部关闭，必须显式启用一个。仅支持非 wasm 目标。
//!
//! 代理不会被隐式启用：[`HttpClient::new`] 完全忽略代理，需要指代理由
//! [`HttpClient::with_proxy`] 指定，需要跟随环境变量与系统代理设置时用
//! [`HttpClient::with_system_proxy`]。
//!
//! 重试只包在单个请求外面，不会重放调用方发起的业务请求：下载与探测遇到限流
//! （429）或瞬时故障（502、503、504）时自行退避重试，分块下载只补没写完的部分。
#[cfg(not(any(feature = "reqwest", feature = "wreq")))]
compile_error!("framework-http 需要启用后端 feature：`reqwest` 或 `wreq`");

mod backend;
mod client;
mod download;
mod header;
mod method;
mod request;
mod response;
mod retry;

pub use client::HttpClient;
pub use download::{DEFAULT_THREADS, DownloadOutcome, RemoteProbe};
pub use method::HttpMethod;
pub use request::HttpRequest;
pub use response::HttpResponse;

use anyhow::{Result, anyhow};
use std::future::Future;
use std::sync::Arc;

/// Axum 服务上下文。
#[derive(Debug, Clone)]
pub struct AxumContext {
    /// 绑定地址。
    pub host: String,
    /// 实际绑定端口，传入随机端口时用于感知真实端口。
    pub port: u16,
}

impl AxumContext {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    /// 服务地址，形如 `127.0.0.1:8080`。
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

tokio::task_local! {
    static AXUM_CONTEXT: Arc<AxumContext>;
}

pub async fn scope_axum<F>(context: Arc<AxumContext>, future: F) -> F::Output
where
    F: Future,
{
    AXUM_CONTEXT.scope(context, future).await
}

pub fn use_axum() -> Result<Arc<AxumContext>> {
    AXUM_CONTEXT
        .try_with(Arc::clone)
        .map_err(|error| anyhow!("当前调用不在 Axum 上下文作用域内：{error}"))
}

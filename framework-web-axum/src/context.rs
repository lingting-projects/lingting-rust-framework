use crate::{AxumShared, AxumStatus};
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
    shared: Arc<AxumShared>,
}

impl AxumContext {
    /// 创建独立上下文，状态恒为未运行，不与任何服务实例共享。
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self::with_shared(host, port, AxumShared::new())
    }

    pub(crate) fn with_shared(host: impl Into<String>, port: u16, shared: Arc<AxumShared>) -> Self {
        Self {
            host: host.into(),
            port,
            shared,
        }
    }

    /// 服务地址，形如 `127.0.0.1:8080`。
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// 当前运行状态。
    pub fn status(&self) -> AxumStatus {
        self.shared.status()
    }

    /// 等待服务进入停止流程（停止中或已停止），返回进入时的状态。
    ///
    /// 服务未启动或运行中时一直等待；已处于停止流程时立即返回，因此可在收到通知后回收资源。
    pub async fn subscribe_stopping(&self) -> AxumStatus {
        let mut receiver = self.shared.subscribe();
        loop {
            let status = *receiver.borrow_and_update();
            if status.is_stopping() {
                return status;
            }
            if receiver.changed().await.is_err() {
                return AxumStatus::Stopped;
            }
        }
    }

    pub(crate) fn shared(&self) -> &Arc<AxumShared> {
        &self.shared
    }

    pub(crate) fn set_status(&self, status: AxumStatus) {
        self.shared.set_status(status);
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

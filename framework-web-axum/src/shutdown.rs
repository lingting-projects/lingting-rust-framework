use crate::AxumShared;
use log::info;
use std::sync::Arc;

/// 优雅关闭句柄，`Clone` 后可在任意任务、路由或定时器中触发关闭。
#[derive(Debug, Clone)]
pub struct AxumShutdown(pub(crate) Arc<AxumShared>);

impl AxumShutdown {
    pub(crate) fn new(shared: Arc<AxumShared>) -> Self {
        Self(shared)
    }

    /// 触发优雅关闭：服务进入停止流程，不再处理新请求，等待进行中的请求结束后退出。
    ///
    /// 幂等，多次调用无副作用；已处于停止流程时不做任何事。
    pub fn shutdown(&self) {
        if self.0.stop() {
            info!("Axum 服务进入停止流程，等待进行中的请求结束");
        }
    }

    /// 是否已进入停止流程（停止中或已停止）。
    pub fn is_shutdown(&self) -> bool {
        self.0.status().is_stopping()
    }
}

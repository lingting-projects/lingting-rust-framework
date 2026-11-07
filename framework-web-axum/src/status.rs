use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use tokio::sync::watch;

/// Axum 服务运行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxumStatus {
    /// 未运行：已绑定端口，尚未启动。
    Unstarted,
    /// 启动中：正在组装路由、准备开始服务。
    Starting,
    /// 运行中：正常接受请求。
    Running,
    /// 停止中：已触发优雅关闭，不再处理新请求，等待进行中的请求结束。
    Stopping,
    /// 已停止：服务已退出。
    Stopped,
}

impl AxumStatus {
    /// 是否正在运行（仅 [`AxumStatus::Running`]）。
    pub fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }

    /// 是否处于停止流程（[`AxumStatus::Stopping`] 或 [`AxumStatus::Stopped`]）。
    pub fn is_stopping(self) -> bool {
        matches!(self, Self::Stopping | Self::Stopped)
    }
}

/// 运行状态与关闭通知的共享状态，由 [`crate::AxumContext`] 与 [`crate::AxumShutdown`] 共同持有。
pub(crate) struct AxumShared {
    status: watch::Sender<AxumStatus>,
}

impl AxumShared {
    pub(crate) fn new() -> Arc<Self> {
        let (status, _) = watch::channel(AxumStatus::Unstarted);
        Arc::new(Self { status })
    }

    /// 当前状态。
    pub(crate) fn status(&self) -> AxumStatus {
        *self.status.borrow()
    }

    /// 推进状态，仅由服务内部调用。
    pub(crate) fn set_status(&self, status: AxumStatus) {
        self.status.send_replace(status);
    }

    /// 进入停止中，返回是否由本次调用完成状态迁移。
    pub(crate) fn stop(&self) -> bool {
        self.status.send_if_modified(|status| {
            if status.is_stopping() {
                return false;
            }
            *status = AxumStatus::Stopping;
            true
        })
    }

    /// 订阅状态变化。
    pub(crate) fn subscribe(&self) -> watch::Receiver<AxumStatus> {
        self.status.subscribe()
    }
}

impl Debug for AxumShared {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AxumShared")
            .field("status", &self.status())
            .finish()
    }
}

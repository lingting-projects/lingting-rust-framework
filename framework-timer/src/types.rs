use anyhow::Error;
use std::future::Future;
use std::pin::Pin;

/// 定时任务执行后的调度结果。
pub struct TimerResult {
    /// 任务执行业务错误；存在时会记录警告日志。
    pub error: Option<Error>,
    /// 下次执行的毫秒时间戳；未指定时使用宏声明的间隔。
    pub next: Option<i64>,
}

impl TimerResult {
    /// 表示任务成功完成并使用默认执行间隔。
    pub const fn success() -> Self {
        Self {
            error: None,
            next: None,
        }
    }

    /// 表示任务完成并在指定毫秒时间戳再次执行。
    pub const fn with_next(next: i64) -> Self {
        Self {
            error: None,
            next: Some(next),
        }
    }

    /// 表示任务失败并使用默认执行间隔。
    pub fn failure(error: Error) -> Self {
        Self {
            error: Some(error),
            next: None,
        }
    }
}

/// 已装箱的定时任务 Future。
pub type TimerFuture = Pin<Box<dyn Future<Output = TimerResult> + Send>>;

/// 可被定时器调度的无参异步任务。
pub type TimerTarget = fn() -> TimerFuture;

/// 为任务建立外部异步上下文的执行包装器。
pub type TimerWrapper = std::sync::Arc<dyn Fn(TimerTarget) -> TimerFuture + Send + Sync>;

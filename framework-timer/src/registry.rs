use crate::TimerTarget;

/// 由 `#[timer]` 生成并收集的任务元数据。
pub struct TimerRegistration {
    /// 任务在日志中使用的唯一名称。
    pub name: &'static str,
    /// 同时运行的任务实例数。
    pub count: usize,
    /// 任务默认执行间隔，单位为毫秒。
    pub interval: i64,
    /// 此任务 panic 后是否立即重试。
    pub retry_immediately_on_panic: Option<bool>,
    /// 任务入口。
    pub target: TimerTarget,
}

inventory::collect!(TimerRegistration);

/// 向链接期全局注册表提交一个定时任务。
pub use inventory::submit as push_timer;

/// 获取当前链接到进程的全部定时任务。
pub fn timer_iter() -> impl Iterator<Item = &'static TimerRegistration> {
    inventory::iter::<TimerRegistration>.into_iter()
}

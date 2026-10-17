/// 可被定时任务运行时按名称匹配的类型。
///
/// 已为字符串类型与 `generate_name` 生成的枚举提供实现，因此 [`crate::wake`]
/// 既能接受 `"task_name"` 这样的字面量，也能接受生成的名称枚举。
pub trait TimerNameLike {
    /// 返回与 [`crate::TimerRegistration::name`] 一致的原始任务函数名。
    fn timer_name(&self) -> &str;
}

impl TimerNameLike for &str {
    fn timer_name(&self) -> &str {
        self
    }
}

impl TimerNameLike for String {
    fn timer_name(&self) -> &str {
        self
    }
}

impl TimerNameLike for &String {
    fn timer_name(&self) -> &str {
        self
    }
}

impl TimerNameLike for std::borrow::Cow<'_, str> {
    fn timer_name(&self) -> &str {
        self
    }
}

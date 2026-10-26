/// 分发者：决定处理函数在哪里、何时执行。
///
/// 总线本身不起线程、不依赖任何 runtime，处理函数的执行位置完全由本 trait 的实现决定。
pub trait Dispatcher: Send + Sync + 'static {
    /// 提交一个任务。
    ///
    /// 实现可以在当前线程立即执行，也可以投递到线程池或某个 executor。
    /// 约定：不应长时间阻塞调用者；任务应当最终被执行或被丢弃（丢弃是允许的）。
    fn dispatch(&self, job: Job);
}

/// 待执行的一次处理函数调用。
///
/// 由总线在 `notify` 时创建并交给 [`Dispatcher`]，实现方只负责调用 [`Job::run`]，
/// 或者直接丢弃（丢弃时总线的合并标记会被复位，该订阅之后仍能再次触发）。
pub struct Job {
    task: Option<Box<dyn FnOnce() + Send + 'static>>,
    /// 任务未执行即被丢弃时执行的复位动作。
    reset: Option<Box<dyn FnOnce() + Send + 'static>>,
}

impl Job {
    pub(crate) fn new(
        task: Box<dyn FnOnce() + Send + 'static>,
        reset: Box<dyn FnOnce() + Send + 'static>,
    ) -> Self {
        Self {
            task: Some(task),
            reset: Some(reset),
        }
    }

    /// 执行任务。
    ///
    /// 处理函数自身的 panic 已在内部捕获并记录日志，不会传播给调用者。
    pub fn run(mut self) {
        let Some(task) = self.task.take() else {
            return;
        };
        // 任务会先复位合并标记再执行处理函数，因此丢弃复位守卫即可
        drop(self.reset.take());
        task();
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        if let Some(reset) = self.reset.take() {
            reset();
        }
    }
}

/// 默认分发者：在调用 [`crate::Bus::notify`] 的线程内同步执行处理函数。
pub struct InlineDispatcher;

impl Dispatcher for InlineDispatcher {
    fn dispatch(&self, job: Job) {
        job.run();
    }
}

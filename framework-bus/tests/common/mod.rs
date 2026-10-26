#![allow(dead_code)]

use framework_bus::{Dispatcher, Job};
use std::sync::{Mutex, MutexGuard, Once};

/// 串行化需要断言日志的测试，避免同一测试二进制内的并发测试互相干扰。
pub fn serialize() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());

    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// 只入队不执行的分发者，用于观察任务产生与合并的时机。
#[derive(Default)]
pub struct QueueDispatcher {
    jobs: Mutex<Vec<Job>>,
}

impl QueueDispatcher {
    pub fn len(&self) -> usize {
        self.jobs.lock().unwrap().len()
    }

    /// 取出全部任务，由调用方决定是否执行。
    pub fn take(&self) -> Vec<Job> {
        std::mem::take(&mut *self.jobs.lock().unwrap())
    }

    /// 依次执行全部任务。
    pub fn run_all(&self) {
        for job in self.take() {
            job.run();
        }
    }

    /// 丢弃全部任务，不执行。
    pub fn drop_all(&self) {
        drop(self.take());
    }
}

impl Dispatcher for QueueDispatcher {
    fn dispatch(&self, job: Job) {
        self.jobs.lock().unwrap().push(job);
    }
}

/// 记录被提交过任务的分发者，用于验证 `set_dispatcher` 的切换。
#[derive(Default)]
pub struct CountingDispatcher {
    count: Mutex<usize>,
}

impl CountingDispatcher {
    pub fn count(&self) -> usize {
        *self.count.lock().unwrap()
    }
}

impl Dispatcher for CountingDispatcher {
    fn dispatch(&self, job: Job) {
        *self.count.lock().unwrap() += 1;
        job.run();
    }
}

pub struct LogCollector {
    records: Mutex<Vec<(log::Level, String)>>,
}

static COLLECTOR: LogCollector = LogCollector {
    records: Mutex::new(Vec::new()),
};

impl log::Log for LogCollector {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        let message = format!("{}", record.args());
        self.records.lock().unwrap().push((record.level(), message));
    }

    fn flush(&self) {}
}

/// 安装测试日志收集器，返回收集器本身。
pub fn logs() -> &'static LogCollector {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        log::set_logger(&COLLECTOR).expect("日志收集器只能安装一次");
        log::set_max_level(log::LevelFilter::Trace);
    });

    &COLLECTOR
}

impl LogCollector {
    /// 清空已收集的记录。
    pub fn clear(&self) {
        self.records.lock().unwrap().clear();
    }

    /// 统计包含指定片段的记录数。
    pub fn count_containing(&self, level: log::Level, fragment: &str) -> usize {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|(record_level, message)| *record_level == level && message.contains(fragment))
            .count()
    }
}

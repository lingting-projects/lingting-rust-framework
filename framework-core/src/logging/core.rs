use std::path::PathBuf;

use tracing::{Event, Subscriber};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::{Context, Filter, Layer};
use tracing_subscriber::registry::LookupSpan;

use super::archive::ArchiveWorker;
use super::visitor_sql::{SqlStatementVisitor, TARGET};

/// 日志层统一类型，供各日志文件实现复用。
pub(crate) type BoxLayer = Box<dyn Layer<tracing_subscriber::Registry> + Send + Sync>;

/// 自动归档日志的配置。
#[derive(Debug, Clone, Copy)]
pub struct LogArchiveConfig {
    pub retention_days: u64,
}

/// 日志初始化配置。
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: LevelFilter,
    pub console: bool,
    pub directory: Option<PathBuf>,
    pub combined: bool,
    pub archive: Option<LogArchiveConfig>,
}

impl LoggingConfig {
    /// 创建默认配置，不写入日志文件。
    pub fn new() -> Self {
        Self {
            level: LevelFilter::INFO,
            console: true,
            directory: None,
            combined: false,
            archive: Some(LogArchiveConfig { retention_days: 7 }),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// 日志写入器与归档线程的生命周期守卫。
pub struct LoggingGuard {
    workers: Vec<WorkerGuard>,
    archive: Option<ArchiveWorker>,
}

impl LoggingGuard {
    pub(crate) fn new(workers: Vec<WorkerGuard>, archive: Option<ArchiveWorker>) -> Self {
        Self { workers, archive }
    }
}

impl Drop for LoggingGuard {
    fn drop(&mut self) {
        // 先停止归档线程，再释放日志写入器，避免退出过程继续访问日志目录。
        self.archive = None;
        self.workers.clear();
    }
}

/// 创建控制台日志层。
pub(crate) fn console_layer(level: LevelFilter) -> BoxLayer {
    Box::new(
        fmt::layer()
            .with_ansi(true)
            .with_target(true)
            .with_filter(level)
            .with_filter(DefaultLoggerFilter),
    )
}

/// 过滤无意义日志的公共过滤器，控制台与文件日志层共同使用。
pub(crate) struct DefaultLoggerFilter;

impl<S> Filter<S> for DefaultLoggerFilter
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn enabled(&self, _: &tracing::Metadata<'_>, _: &Context<'_, S>) -> bool {
        true
    }

    fn event_enabled(&self, event: &Event<'_>, _: &Context<'_, S>) -> bool {
        if event.metadata().target() != TARGET {
            return true;
        }

        let mut visitor = SqlStatementVisitor::default();
        event.record(&mut visitor);
        !visitor.is_ignored()
    }
}

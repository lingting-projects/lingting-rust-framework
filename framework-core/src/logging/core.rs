use std::path::PathBuf;
use std::sync::Arc;

use tracing::{Event, Metadata};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::Registry;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::{Context, Filter, Layer};

use super::archive::ArchiveWorker;
use super::filter_sqlx::DefaultSqlxFilter;

/// 外部可扩展的日志过滤器：返回 `false` 表示忽略对应事件。
///
/// 使用 `Arc` 包裹，便于在多个日志层之间复用同一实例。
pub type LoggingFilter = Arc<dyn Filter<Registry> + Send + Sync + 'static>;

/// 日志层统一类型，供各日志文件实现复用。
pub(crate) type BoxLayer = Box<dyn Layer<Registry> + Send + Sync>;

/// 自动归档日志的配置。
#[derive(Debug, Clone, Copy)]
pub struct LogArchiveConfig {
    pub retention_days: u64,
}

/// 日志初始化配置。
#[derive(Clone)]
pub struct LoggingConfig {
    pub level: LevelFilter,
    pub console: bool,
    pub directory: Option<PathBuf>,
    pub combined: bool,
    pub archive: Option<LogArchiveConfig>,
    /// 日志过滤器：任一过滤器返回 `false` 即忽略该事件。
    pub filters: Vec<LoggingFilter>,
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
            filters: vec![Arc::new(DefaultSqlxFilter)],
        }
    }

    /// 追加一个日志过滤器。
    pub fn push_filter(&mut self, filter: LoggingFilter) {
        self.filters.push(filter);
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for LoggingConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LoggingConfig")
            .field("level", &self.level)
            .field("console", &self.console)
            .field("directory", &self.directory)
            .field("combined", &self.combined)
            .field("archive", &self.archive)
            .field("filters", &self.filters.len())
            .finish()
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
pub(crate) fn console_layer(level: LevelFilter, filters: &[LoggingFilter]) -> BoxLayer {
    Box::new(
        fmt::layer()
            .with_ansi(true)
            .with_target(true)
            .with_filter(level)
            .with_filter(CombinedFilter::new(filters)),
    )
}

/// 组合多个日志过滤器：所有过滤器都放行时才记录该事件。
pub(crate) struct CombinedFilter {
    filters: Vec<LoggingFilter>,
}

impl CombinedFilter {
    pub(crate) fn new(filters: &[LoggingFilter]) -> Self {
        Self {
            filters: filters.to_vec(),
        }
    }
}

impl Filter<Registry> for CombinedFilter {
    fn enabled(&self, metadata: &Metadata<'_>, context: &Context<'_, Registry>) -> bool {
        self.filters
            .iter()
            .all(|filter| filter.enabled(metadata, context))
    }

    fn event_enabled(&self, event: &Event<'_>, context: &Context<'_, Registry>) -> bool {
        self.filters
            .iter()
            .all(|filter| filter.event_enabled(event, context))
    }
}

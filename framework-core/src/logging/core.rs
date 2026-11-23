use std::path::PathBuf;
use std::sync::Arc;

use tracing::{Event, Metadata};
#[cfg(not(target_arch = "wasm32"))]
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::Registry;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::{Context, Filter, Layer};

#[cfg(not(target_arch = "wasm32"))]
use super::archive::ArchiveWorker;
use super::filter_sqlx::DefaultSqlxFilter;
#[cfg(not(target_arch = "wasm32"))]
use super::flush::FileFlush;

/// 外部可扩展的日志过滤器：返回 `false` 表示忽略对应事件。
///
/// 使用 `Arc` 包裹，便于在多个日志层之间复用同一实例。
pub type LoggingFilter = Arc<dyn Filter<Registry> + Send + Sync + 'static>;

/// 调用方可追加的自定义日志层（审计层等）：只加计数，不改变事件内容与去向。
pub type LoggingLayer = Box<dyn Layer<Registry> + Send + Sync>;

/// 日志层统一类型，供各日志文件实现复用。
pub(crate) type BoxLayer = LoggingLayer;

/// 自动归档日志的配置。
#[derive(Debug, Clone, Copy)]
pub struct LogArchiveConfig {
    pub retention_days: u64,
}

/// 日志初始化配置。
///
/// 自定义日志层是 `Box`（不可克隆），因此本结构不实现 `Clone`，并由 [`super::init`] 按值消费。
pub struct LoggingConfig {
    pub level: LevelFilter,
    pub console: bool,
    pub directory: Option<PathBuf>,
    pub combined: bool,
    pub archive: Option<LogArchiveConfig>,
    /// 日志过滤器：任一过滤器返回 `false` 即忽略该事件。
    pub filters: Vec<LoggingFilter>,
    /// 调用方追加的自定义日志层：与 console / file 层并列挂到 subscriber 上。
    pub layers: Vec<LoggingLayer>,
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
            layers: Vec::new(),
        }
    }

    /// 追加一个日志过滤器。
    pub fn push_filter(&mut self, filter: LoggingFilter) {
        self.filters.push(filter);
    }

    /// 追加一个自定义日志层：与 console / file 层并列，不参与级别与过滤器的判定。
    pub fn push_layer(&mut self, layer: LoggingLayer) {
        self.layers.push(layer);
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
            .field("layers", &self.layers.len())
            .finish()
    }
}

/// 日志写入器与归档线程的生命周期守卫。
pub struct LoggingGuard {
    #[cfg(not(target_arch = "wasm32"))]
    workers: Vec<WorkerGuard>,
    #[cfg(not(target_arch = "wasm32"))]
    flushes: Vec<FileFlush>,
    #[cfg(not(target_arch = "wasm32"))]
    archive: Option<ArchiveWorker>,
}

impl LoggingGuard {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn new(
        workers: Vec<WorkerGuard>,
        flushes: Vec<FileFlush>,
        archive: Option<ArchiveWorker>,
    ) -> Self {
        Self {
            workers,
            flushes,
            archive,
        }
    }

    /// 立即把日志刷到磁盘。
    ///
    /// 日志是异步写的，正常退出由析构收尾；进程被 panic 中止时走不到析构，
    /// 而最后一条日志往往正是崩溃原因，因此需要在异常退出前主动调用一次。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn flush(&self) {
        for flush in &self.flushes {
            flush.flush();
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
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

use std::io;
use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::{LevelFilter, filter_fn};
use tracing_subscriber::fmt;
use tracing_subscriber::layer::Layer;

use super::core::{BoxLayer, DefaultLoggerFilter};
use super::file;
use super::visitor_logging::{LogDebugFilter, LogStrFilter};

/// 聚合日志文件名。
const FILE_NAME: &str = "combined.log";

/// 创建聚合日志层，仅写入初始化等级允许的日志，并注册对应的写入器守卫。
pub(crate) fn layer(
    directory: &Path,
    max_level: LevelFilter,
    workers: &mut Vec<WorkerGuard>,
    str_filters: &[LogStrFilter],
    debug_filters: &[LogDebugFilter],
) -> io::Result<BoxLayer> {
    let writer = file::current_file_writer(directory, FILE_NAME, workers)?;
    Ok(Box::new(
        fmt::layer()
            .with_ansi(false)
            .with_target(true)
            .with_writer(writer)
            .with_filter(filter_fn(move |metadata| metadata.level() <= &max_level))
            .with_filter(DefaultLoggerFilter::new(
                str_filters.to_vec(),
                debug_filters.to_vec(),
            )),
    ))
}

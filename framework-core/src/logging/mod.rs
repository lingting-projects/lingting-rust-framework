//! 日志初始化模块：控制台输出、按级别与聚合的文件日志、按天切分与过期归档。

mod archive;
mod combined;
mod core;
mod file;
mod visitor_sql;

use std::fs;
use std::io;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub use core::{LogArchiveConfig, LoggingConfig, LoggingGuard};

/// 初始化 tracing 日志订阅器。
///
/// 返回的 [`LoggingGuard`] 必须存活到进程退出，否则日志写入器与归档线程会被提前释放。
pub fn init(config: &LoggingConfig) -> io::Result<LoggingGuard> {
    let mut workers = Vec::new();
    let mut layers = Vec::new();
    let mut archive = None;

    if config.console {
        layers.push(core::console_layer(config.level));
    }

    if let Some(directory) = config.directory.as_deref() {
        fs::create_dir_all(directory)?;
        layers.extend(file::layers(directory, config.level, &mut workers)?);
        if config.combined {
            layers.push(combined::layer(directory, config.level, &mut workers)?);
        }
        if let Some(archive_config) = config.archive {
            archive = Some(archive::ArchiveWorker::start(
                directory.to_owned(),
                archive_config,
            ));
        }
    }

    tracing_subscriber::registry()
        .with(layers)
        .try_init()
        .map_err(io::Error::other)?;

    Ok(LoggingGuard::new(workers, archive))
}

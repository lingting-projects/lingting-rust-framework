//! 日志初始化模块：控制台输出、按级别与聚合的文件日志、按天切分与过期归档。

#[cfg(not(target_arch = "wasm32"))]
mod archive;
#[cfg(not(target_arch = "wasm32"))]
mod combined;
mod core;
#[cfg(not(target_arch = "wasm32"))]
mod file;
mod filter_sqlx;

#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::io;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub use core::{LogArchiveConfig, LoggingConfig, LoggingFilter, LoggingGuard};
pub use filter_sqlx::DefaultSqlxFilter;

/// 初始化 tracing 日志订阅器。
///
/// 返回的 [`LoggingGuard`] 必须存活到进程退出，否则日志写入器与归档线程会被提前释放。
pub fn init(config: &LoggingConfig) -> io::Result<LoggingGuard> {
    let mut layers = Vec::new();

    if config.console {
        layers.push(core::console_layer(config.level, &config.filters));
    }

    #[cfg(not(target_arch = "wasm32"))]
    let guard = {
        let mut workers = Vec::new();
        let mut archive = None;

        if let Some(directory) = config.directory.as_deref() {
            fs::create_dir_all(directory)?;
            layers.extend(file::layers(
                directory,
                config.level,
                &mut workers,
                &config.filters,
            )?);
            if config.combined {
                layers.push(combined::layer(
                    directory,
                    config.level,
                    &mut workers,
                    &config.filters,
                )?);
            }
            if let Some(archive_config) = config.archive {
                archive = Some(archive::ArchiveWorker::start(
                    directory.to_owned(),
                    archive_config,
                ));
            }
        }

        LoggingGuard::new(workers, archive)
    };

    #[cfg(target_arch = "wasm32")]
    let guard = LoggingGuard {};

    tracing_subscriber::registry()
        .with(layers)
        .try_init()
        .map_err(io::Error::other)?;

    Ok(guard)
}

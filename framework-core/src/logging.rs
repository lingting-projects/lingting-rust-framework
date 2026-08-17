use anyhow::{Result, anyhow};
use chrono::Local;
use log::{LevelFilter, Log, Metadata, Record};
use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static LOGGER: OnceLock<&'static CoreLogger> = OnceLock::new();
static INIT_LOCK: Mutex<()> = Mutex::new(());

/// 日志初始化配置。
pub struct LoggingConfig {
    /// 允许输出的最大日志等级。
    pub max_level: LevelFilter,
    /// 日志文件所在文件夹。未提供时仅输出到标准输出。
    pub directory: Option<PathBuf>,
}

struct CoreLogger {
    max_level: LevelFilter,
    writer: Mutex<LogWriter>,
}

struct LogWriter {
    stdout: std::io::Stdout,
    file: Option<BufWriter<File>>,
}

impl CoreLogger {
    fn new(config: &LoggingConfig) -> Result<Self> {
        let file = config
            .directory
            .as_ref()
            .map(|directory| {
                create_dir_all(directory)?;
                File::options()
                    .create(true)
                    .append(true)
                    .open(directory.join("relayx.log"))
            })
            .transpose()?;
        Ok(Self {
            max_level: config.max_level,
            writer: Mutex::new(LogWriter {
                stdout: std::io::stdout(),
                file: file.map(BufWriter::new),
            }),
        })
    }
}

impl Log for CoreLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.max_level >= metadata.level().to_level_filter()
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let source = match (record.file(), record.line()) {
            (Some(file), Some(line)) => format!("{file}:{line}"),
            (Some(file), None) => file.to_string(),
            _ => "未知位置".to_string(),
        };
        let message = format!(
            "[{} {:<5} {} {}] {}\n",
            Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            record.level(),
            record.target(),
            source,
            record.args()
        );
        let Ok(mut writer) = self.writer.lock() else {
            return;
        };
        let _ = writer.stdout.write_all(message.as_bytes());
        let _ = writer.stdout.flush();
        if let Some(file) = &mut writer.file {
            let _ = file.write_all(message.as_bytes());
            let _ = file.flush();
        }
    }

    fn flush(&self) {
        let Ok(mut writer) = self.writer.lock() else {
            return;
        };
        let _ = writer.stdout.flush();
        if let Some(file) = &mut writer.file {
            let _ = file.flush();
        }
    }
}

/// 初始化全局日志。
///
/// 首次调用安装日志实现；后续调用保留首次配置并直接返回成功。
pub fn init(config: &LoggingConfig) -> Result<()> {
    let _guard = INIT_LOCK
        .lock()
        .map_err(|_| anyhow!("日志初始化锁已中毒"))?;
    if LOGGER.get().is_some() {
        return Ok(());
    }
    let logger = Box::leak(Box::new(CoreLogger::new(config)?));
    log::set_logger(logger).map_err(|error| anyhow!(error))?;
    LOGGER.set(logger).map_err(|_| anyhow!("日志已初始化"))?;
    log::set_max_level(config.max_level);
    Ok(())
}

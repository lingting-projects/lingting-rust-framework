use chrono::Local;
use log::{LevelFilter, Log, Metadata, Record};
use std::error::Error;
use std::fmt::{self, Write as _};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::path::Path;
use std::sync::Mutex;

struct ApplicationLogger {
    file: Option<Mutex<File>>,
}

impl Log for ApplicationLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        !metadata.target().starts_with("sqlx")
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let message = format_record(record);
        let _ = writeln!(io::stdout().lock(), "{message}");

        if let Some(file) = &self.file
            && let Ok(mut file) = file.lock()
        {
            let _ = writeln!(file, "{message}");
        }
    }

    fn flush(&self) {
        let _ = io::stdout().lock().flush();
        if let Some(file) = &self.file
            && let Ok(mut file) = file.lock()
        {
            let _ = file.flush();
        }
    }
}

#[derive(Debug)]
pub enum LogInitError {
    CreateDirectory(io::Error),
    OpenFile(io::Error),
    SetLogger(log::SetLoggerError),
}

impl fmt::Display for LogInitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateDirectory(error) => write!(formatter, "创建日志目录失败：{error}"),
            Self::OpenFile(error) => write!(formatter, "打开日志文件失败：{error}"),
            Self::SetLogger(error) => write!(formatter, "设置全局日志器失败：{error}"),
        }
    }
}

impl Error for LogInitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateDirectory(error) | Self::OpenFile(error) => Some(error),
            Self::SetLogger(_) => None,
        }
    }
}

pub fn init_logger(log_dir: impl AsRef<Path>, enable_file: bool) -> Result<(), LogInitError> {
    let file = if enable_file {
        let log_dir = log_dir.as_ref();
        fs::create_dir_all(log_dir).map_err(LogInitError::CreateDirectory)?;
        let file_name = format!("relayx-anubis-{}.log", Local::now().format("%Y-%m-%d"));
        Some(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_dir.join(file_name))
                .map_err(LogInitError::OpenFile)?,
        )
    } else {
        None
    };

    let logger = Box::leak(Box::new(ApplicationLogger {
        file: file.map(Mutex::new),
    }));
    log::set_logger(logger).map_err(LogInitError::SetLogger)?;
    #[cfg(debug_assertions)]
    log::set_max_level(LevelFilter::Trace);
    #[cfg(not(debug_assertions))]
    log::set_max_level(LevelFilter::Info);
    Ok(())
}

fn format_record(record: &Record<'_>) -> String {
    let mut message = format!(
        "{} {:<5}",
        Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        record.level()
    );

    if let Some(module) = record.module_path() {
        let _ = write!(message, " [{module}");
        if let Some(file) = record.file() {
            let _ = write!(message, " {file}");
            if let Some(line) = record.line() {
                let _ = write!(message, ":{line}");
            }
        }
        message.push(']');
    }

    let _ = write!(message, " {}", record.args());
    message
}

use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use chrono::{Local, NaiveDate};
use flate2::Compression;
use flate2::write::GzEncoder;

use super::core::LogArchiveConfig;

/// 归档目录名称。
const DIRECTORY_NAME: &str = "archive";
/// 归档线程的清理间隔。
const CLEAN_INTERVAL: Duration = Duration::from_secs(60 * 60);
/// 归档文件名中的日期长度，形如 `2026-02-14`。
const DATE_LENGTH: usize = 10;
/// 归档文件名中的日期格式。
const DATE_FORMAT: &str = "%Y-%m-%d";

/// 归档线程句柄，释放时通知并等待线程退出。
pub(crate) struct ArchiveWorker {
    stop: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl ArchiveWorker {
    /// 启动归档线程，启动时立即执行一次过期日志清理。
    pub(crate) fn start(directory: PathBuf, config: LogArchiveConfig) -> Self {
        let (stop, receiver) = mpsc::channel();
        let thread = thread::spawn(move || {
            clean_expired(&directory, config.retention_days);
            loop {
                match receiver.recv_timeout(CLEAN_INTERVAL) {
                    Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                    Err(RecvTimeoutError::Timeout) => {
                        clean_expired(&directory, config.retention_days);
                    }
                }
            }
        });
        Self {
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for ArchiveWorker {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// 归档目录路径，供日志写入器与归档线程复用。
pub(crate) fn archive_directory(directory: &Path) -> PathBuf {
    directory.join(DIRECTORY_NAME)
}

/// 归档文件名，形如 `2026-02-14-info.gz`。
pub(crate) fn file_name(date: NaiveDate, file_name: &str) -> String {
    let name = file_name.strip_suffix(".log").unwrap_or(file_name);
    format!("{date}-{name}.gz")
}

/// 压缩日志文件为归档文件，成功后删除原日志文件；空日志文件不生成归档，直接删除。
pub(crate) fn compress_log(log_path: &Path, archive_path: &Path) -> io::Result<()> {
    if fs::metadata(log_path)?.len() == 0 {
        return fs::remove_file(log_path);
    }

    if archive_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "日志归档文件已存在",
        ));
    }

    let input = File::open(log_path)?;
    let output = File::create(archive_path)?;
    let mut encoder = GzEncoder::new(output, Compression::default());
    if io::copy(&mut BufReader::new(input), &mut encoder).is_ok() && encoder.finish().is_ok() {
        fs::remove_file(log_path)
    } else {
        let _ = fs::remove_file(archive_path);
        Err(io::Error::other("压缩日志失败"))
    }
}

/// 删除归档目录中超出保留天数的日志文件。
fn clean_expired(logs_directory: &Path, retention_days: u64) {
    let target = archive_directory(logs_directory);
    if fs::create_dir_all(&target).is_err() {
        return;
    }

    let Ok(entries) = fs::read_dir(&target) else {
        return;
    };
    let today = Local::now().date_naive();

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(date) = log_date(&path) else {
            continue;
        };
        if today.signed_duration_since(date).num_days() >= retention_days as i64 {
            let _ = fs::remove_file(path);
        }
    }
}

/// 从归档文件名解析日志日期。
fn log_date(path: &Path) -> Option<NaiveDate> {
    let name = path.file_name()?.to_str()?.strip_suffix(".gz")?;
    NaiveDate::parse_from_str(name.get(..DATE_LENGTH)?, DATE_FORMAT).ok()
}

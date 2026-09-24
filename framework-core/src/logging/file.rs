use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{Local, NaiveDate};
use tracing::Level;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::filter::{LevelFilter, filter_fn};
use tracing_subscriber::fmt;
use tracing_subscriber::layer::Layer;

use super::archive;
use super::core::{BoxLayer, DefaultLoggerFilter};
use super::visitor_logging::{LogDebugFilter, LogStrFilter};

/// 单级别日志文件与其过滤级别。
const LEVEL_FILES: [(&str, Level); 5] = [
    ("error.log", Level::ERROR),
    ("warn.log", Level::WARN),
    ("info.log", Level::INFO),
    ("debug.log", Level::DEBUG),
    ("trace.log", Level::TRACE),
];

/// 创建全部单级别日志层，各层仅写入初始化等级允许的日志，并注册对应的写入器守卫。
pub(crate) fn layers(
    directory: &Path,
    max_level: LevelFilter,
    workers: &mut Vec<WorkerGuard>,
    str_filters: &[LogStrFilter],
    debug_filters: &[LogDebugFilter],
) -> io::Result<Vec<BoxLayer>> {
    LEVEL_FILES
        .iter()
        .map(|(file_name, level)| {
            layer(
                directory,
                file_name,
                *level,
                max_level,
                workers,
                str_filters,
                debug_filters,
            )
        })
        .collect()
}

/// 创建按天切分的日志写入器，供单级别日志层与聚合日志层复用。
pub(crate) fn current_file_writer(
    directory: &Path,
    file_name: &str,
    workers: &mut Vec<WorkerGuard>,
) -> io::Result<NonBlocking> {
    let writer = CurrentFileWriter::new(directory.to_owned(), file_name)?;
    let (writer, guard) = tracing_appender::non_blocking(writer);
    workers.push(guard);
    Ok(writer)
}

fn layer(
    directory: &Path,
    file_name: &str,
    level: Level,
    max_level: LevelFilter,
    workers: &mut Vec<WorkerGuard>,
    str_filters: &[LogStrFilter],
    debug_filters: &[LogDebugFilter],
) -> io::Result<BoxLayer> {
    let writer = current_file_writer(directory, file_name, workers)?;
    Ok(Box::new(
        fmt::layer()
            .with_ansi(false)
            .with_target(true)
            .with_writer(writer)
            .with_filter(filter_fn(move |metadata| {
                *metadata.level() == level && metadata.level() <= &max_level
            }))
            .with_filter(DefaultLoggerFilter::new(
                str_filters.to_vec(),
                debug_filters.to_vec(),
            )),
    ))
}

struct CurrentFileWriter {
    state: Arc<Mutex<CurrentFileState>>,
}

impl CurrentFileWriter {
    fn new(directory: PathBuf, file_name: &str) -> io::Result<Self> {
        let state = CurrentFileState::new(directory, file_name, Local::now().date_naive())?;
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
        })
    }
}

impl Write for CurrentFileWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("日志写入器已损坏"))?;
        state.rotate_if_needed(Local::now().date_naive())?;
        state
            .file
            .as_mut()
            .ok_or_else(|| io::Error::other("日志文件未打开"))?
            .write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("日志写入器已损坏"))?;
        state
            .file
            .as_mut()
            .ok_or_else(|| io::Error::other("日志文件未打开"))?
            .flush()
    }
}

struct CurrentFileState {
    directory: PathBuf,
    archive_directory: PathBuf,
    file_name: String,
    date: NaiveDate,
    file: Option<File>,
}

impl CurrentFileState {
    fn new(directory: PathBuf, file_name: &str, date: NaiveDate) -> io::Result<Self> {
        let archive_directory = archive::archive_directory(&directory);
        fs::create_dir_all(&archive_directory)?;
        let file = open_current_log(&directory, file_name)?;
        Ok(Self {
            directory,
            archive_directory,
            file_name: file_name.to_owned(),
            date,
            file: Some(file),
        })
    }

    fn rotate_if_needed(&mut self, date: NaiveDate) -> io::Result<()> {
        if self.date == date {
            return Ok(());
        }

        let file = self
            .file
            .take()
            .ok_or_else(|| io::Error::other("日志文件未打开"))?;
        file.sync_all()?;
        drop(file);
        let old_log_path = self.directory.join(&self.file_name);
        let archive_path = self
            .archive_directory
            .join(archive::file_name(self.date, &self.file_name));
        if let Err(error) = archive::compress_log(&old_log_path, &archive_path) {
            self.file = Some(open_current_log(&self.directory, &self.file_name)?);
            return Err(error);
        }
        self.file = Some(open_current_log(&self.directory, &self.file_name)?);
        self.date = date;
        Ok(())
    }
}

fn open_current_log(directory: &Path, file_name: &str) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join(file_name))
}

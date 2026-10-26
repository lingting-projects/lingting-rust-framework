//! 日志落盘。
//!
//! 日志写入走 `tracing_appender` 的异步写入器：调用方只把内容塞进通道，
//! 真正写文件的是后台线程。进程正常退出时由 `WorkerGuard` 的析构收尾，
//! 但 panic 走到 `abort`、或进程被强制结束时析构不会运行，
//! 通道里最后一批日志就写不进文件——而这一批往往正是崩溃原因。
//!
//! 这里的做法是持有一份与写入器相同的日志文件状态，由调用方在异常退出前显式刷盘。

use std::io::Write as _;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::file::CurrentFileState;

/// 主动落盘时等待后台写入线程的上限。
const FLUSH_TIMEOUT: Duration = Duration::from_millis(500);

/// 后台线程还没有放开文件状态时的重试间隔。
const FLUSH_INTERVAL: Duration = Duration::from_millis(5);

/// 单个日志文件的落盘句柄。
///
/// 与写入器共享同一份文件状态：写入器的后台线程持有锁写文件，
/// 这里拿不到锁就说明它正在写，等它写完再刷即可。
pub(crate) struct FileFlush {
    state: Arc<Mutex<CurrentFileState>>,
}

impl FileFlush {
    /// 创建落盘句柄。
    pub(crate) fn new(state: Arc<Mutex<CurrentFileState>>) -> Self {
        Self { state }
    }

    /// 把已经写进文件的日志刷到磁盘。
    ///
    /// 超时只为避免退出路径卡死：超时后放弃，不阻塞进程退出。
    pub(crate) fn flush(&self) {
        let deadline = Instant::now() + FLUSH_TIMEOUT;
        while Instant::now() < deadline {
            let Ok(mut state) = self.state.try_lock() else {
                thread::sleep(FLUSH_INTERVAL);
                continue;
            };
            let Some(file) = state.file.as_mut() else {
                return;
            };
            let _ = file.flush();
            return;
        }
    }
}

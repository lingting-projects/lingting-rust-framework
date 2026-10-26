use std::error::Error;
use std::fmt::{Display, Formatter};

/// 总线已关闭。
///
/// 由 [`Watcher::changed`](crate::Watcher::changed) 返回：总线被显式关闭，
/// 或所有 [`Bus`](crate::Bus) 克隆都已释放。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Closed;

impl Display for Closed {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("framework-bus: bus closed")
    }
}

impl Error for Closed {}

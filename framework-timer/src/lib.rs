//! 基于 Tokio 的进程内定时任务运行时。
//!
//! 任务通过 [`timer`] 宏注册，名称固定为任务函数名。唤醒任务时既可以传字符串，
//! 也可以传入由 `generate` feature 的 [`generate_name::generate`] 生成的名称枚举，
//! 两者都实现了 [`TimerNameLike`]。

#[cfg(feature = "generate")]
pub mod generate_name;
mod name;
mod registry;
mod runtime;
mod types;

pub use framework_proc_timer::timer;
pub use name::TimerNameLike;
pub use registry::{TimerRegistration, push_timer, timer_iter};
pub use runtime::{TimerOption, start, wake};
pub use types::{TimerFuture, TimerResult, TimerTarget, TimerWrapper};

#[doc(hidden)]
pub use inventory;

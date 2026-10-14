//! 操作系统能力：提权判断与外部命令扩展。
//!
//! 支持 Windows、Linux、macOS；各平台实现按编译目标门控，非目标平台的代码与依赖不参与编译。
//!
//! - [`is_elevated`]：判断当前进程是否以管理员（Windows）或 root（Linux、macOS）身份运行。
//! - [`CommandExt`]：`std::process::Command` 的扩展，统一命令的执行、结果获取与错误信息。

pub mod command;

mod is_elevated;

pub use command::CommandExt;
pub use is_elevated::is_elevated;

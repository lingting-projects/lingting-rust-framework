//! 开机自启的注册、注销与启停。
//!
//! 对外只暴露统一配置 [`StartupConfig`] 与统一入口 [`Startup`]，各操作系统的差异由本 crate 内部承担：
//!
//! - Windows：任务计划程序（`schtasks`）
//! - Linux：systemd（系统级 `/etc/systemd/system`，用户级 `~/.config/systemd/user`）
//! - macOS：launchd（系统级 `/Library/LaunchDaemons`，用户级 `~/Library/LaunchAgents`）
//!
//! 各平台的实现按编译目标门控，非目标平台的代码与依赖不参与编译。

pub mod config;
pub mod startup;

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
mod command;

mod platform;
mod scope;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod directory;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod file;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

pub use config::StartupConfig;
pub use startup::Startup;

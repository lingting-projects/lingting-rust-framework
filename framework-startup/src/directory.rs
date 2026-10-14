//! 自启定义文件所在目录。

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::scope::Scope;

/// systemd 系统级单元目录。
#[cfg(target_os = "linux")]
pub(crate) const SYSTEM_UNIT_DIRECTORY: &str = "/etc/systemd/system";

/// launchd 系统级守护进程目录。
#[cfg(target_os = "macos")]
pub(crate) const SYSTEM_PLIST_DIRECTORY: &str = "/Library/LaunchDaemons";

/// 当前用户的主目录。
pub(crate) fn home_directory() -> Result<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .context("未设置 HOME 环境变量")
}

/// systemd 用户级单元目录。
#[cfg(target_os = "linux")]
pub(crate) fn user_unit_directory() -> Result<PathBuf> {
    Ok(home_directory()?.join(".config/systemd/user"))
}

/// launchd 用户级代理目录。
#[cfg(target_os = "macos")]
pub(crate) fn user_plist_directory() -> Result<PathBuf> {
    Ok(home_directory()?.join("Library/LaunchAgents"))
}

/// 按级别选择目录；用户级需要读取主目录，因此可能失败。
pub(crate) fn directory(
    root: &str,
    user: fn() -> Result<PathBuf>,
    scope: Scope,
) -> Result<PathBuf> {
    match scope {
        Scope::Root => Ok(PathBuf::from(root)),
        Scope::User => user(),
    }
}

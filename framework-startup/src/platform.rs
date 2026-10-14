//! 各操作系统的自启实现。

use anyhow::Result;

use crate::config::StartupConfig;
use crate::scope::Scope;

/// 各操作系统的自启实现。
pub(crate) trait Platform {
    /// 查询自启是否已注册。
    fn exists(&self, config: &StartupConfig, scope: Scope) -> Result<bool>;

    /// 查询自启的进程是否正在运行。
    fn is_run(&self, config: &StartupConfig, scope: Scope) -> Result<bool>;

    /// 注册自启，覆盖已存在的同名自启。
    fn install(&self, config: &StartupConfig, scope: Scope) -> Result<()>;

    /// 注销自启。
    fn uninstall(&self, config: &StartupConfig, scope: Scope) -> Result<()>;

    /// 启动自启的进程。
    fn start(&self, config: &StartupConfig, scope: Scope) -> Result<()>;

    /// 停止自启的进程。
    fn stop(&self, config: &StartupConfig, scope: Scope) -> Result<()>;
}

/// 当前操作系统的实现。
#[cfg(target_os = "windows")]
pub(crate) fn current_platform() -> Result<Box<dyn Platform>> {
    Ok(Box::new(crate::windows::WindowsStartup))
}

/// 当前操作系统的实现。
#[cfg(target_os = "linux")]
pub(crate) fn current_platform() -> Result<Box<dyn Platform>> {
    Ok(Box::new(crate::linux::LinuxStartup))
}

/// 当前操作系统的实现。
#[cfg(target_os = "macos")]
pub(crate) fn current_platform() -> Result<Box<dyn Platform>> {
    Ok(Box::new(crate::macos::MacosStartup))
}

/// 当前操作系统的实现。
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub(crate) fn current_platform() -> Result<Box<dyn Platform>> {
    Err(anyhow::anyhow!("当前操作系统不支持开机自启"))
}

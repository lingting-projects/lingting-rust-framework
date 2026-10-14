//! 统一自启入口。

use anyhow::{Result, anyhow};

use crate::config::StartupConfig;
use crate::platform::{Platform, current_platform};
use crate::scope::Scope;

/// 开机自启入口；按当前操作系统选择实现，并记录注册级别。
pub struct Startup {
    config: StartupConfig,
    scope: Scope,
    platform: Box<dyn Platform>,
}

impl Startup {
    /// 创建系统级自启：随系统启动，不依赖用户登录；注册与启停通常需要管理员或 root 权限。
    pub fn root(config: StartupConfig) -> Result<Self> {
        Self::new(config, Scope::Root)
    }

    /// 创建当前用户级自启：随当前用户登录启动，无需管理员或 root 权限。
    ///
    /// 同一名称在系统级与用户级下互不冲突（systemd 与 launchd 存放于不同目录），
    /// 但 Windows 计划任务名在整个任务树的根目录下唯一，因此系统级与用户级不能使用同名任务。
    pub fn user(config: StartupConfig) -> Result<Self> {
        Self::new(config, Scope::User)
    }

    /// 查询自启是否已注册。
    pub fn exists(&self) -> Result<bool> {
        self.platform.exists(&self.config, self.scope)
    }

    /// 查询自启的进程是否正在运行。
    ///
    /// 未注册时的行为由 [`StartupConfig::ignore_not_installed`] 决定：忽略时返回假，否则报错。
    pub fn is_run(&self) -> Result<bool> {
        if self.not_installed()? {
            return Ok(false);
        }

        self.running()
    }

    /// 注册自启；已注册时不重复执行，直接返回成功。
    pub fn install(&self) -> Result<()> {
        if self.exists()? {
            return Ok(());
        }

        self.platform.install(&self.config, self.scope)
    }

    /// 注册自启并覆盖已存在的同名自启，用于让配置变更生效。
    pub fn install_force(&self) -> Result<()> {
        self.platform.install(&self.config, self.scope)
    }

    /// 注销自启；不停止正在运行的进程，需要先停止时由调用方先调 [`Startup::stop`]。
    ///
    /// 未注册时的行为由 [`StartupConfig::ignore_not_installed`] 决定。
    pub fn uninstall(&self) -> Result<()> {
        if self.not_installed()? {
            return Ok(());
        }

        self.platform.uninstall(&self.config, self.scope)
    }

    /// 立即启动；未注册时与已在运行时都不重复执行，直接返回成功。
    pub fn start(&self) -> Result<()> {
        if self.not_installed()? || self.running()? {
            return Ok(());
        }

        self.platform.start(&self.config, self.scope)
    }

    /// 立即停止；未注册时与未运行时都不重复执行，直接返回成功。
    pub fn stop(&self) -> Result<()> {
        if self.not_installed()? || !self.running()? {
            return Ok(());
        }

        self.platform.stop(&self.config, self.scope)
    }

    /// 重启：先停止再启动。
    ///
    /// 只探测一次运行状态：未运行时直接启动，正在运行时停止后再启动。
    /// 停止阶段的错误只说明进程本来就已退出，不阻断后续启动。
    pub fn restart(&self) -> Result<()> {
        if self.not_installed()? {
            return Ok(());
        }

        if self.running()? {
            let _ = self.platform.stop(&self.config, self.scope);
        }

        self.platform.start(&self.config, self.scope)
    }

    /// 查询运行状态；调用方需先确认自启已注册。
    fn running(&self) -> Result<bool> {
        self.platform.is_run(&self.config, self.scope)
    }

    /// 判断自启未注册；未注册且要求忽略时返回真，未注册且要求报错时返回错误。
    fn not_installed(&self) -> Result<bool> {
        if self.exists()? {
            return Ok(false);
        }
        if self.config.ignore_not_installed {
            return Ok(true);
        }

        Err(anyhow!("自启 {} 未注册", self.config.name))
    }

    fn new(config: StartupConfig, scope: Scope) -> Result<Self> {
        Ok(Self {
            config: config.resolve()?,
            scope,
            platform: current_platform()?,
        })
    }
}

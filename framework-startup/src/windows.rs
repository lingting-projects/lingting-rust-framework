//! Windows 任务计划程序实现：注册计划任务，任务直接启动目标可执行文件。
//!
//! 系统级任务在开机时以 `SYSTEM` 身份运行，用户级任务在用户登录时以当前用户身份运行。

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use framework_system::CommandExt;
use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

use crate::config::StartupConfig;
use crate::platform::Platform;
use crate::scope::Scope;

/// 计划任务名与自启名一致。
pub(crate) struct WindowsStartup;

impl Platform for WindowsStartup {
    fn exists(&self, config: &StartupConfig, _scope: Scope) -> Result<bool> {
        // 任务存在时 query 返回成功，不存在时返回非零退出码。
        Ok(Command::new("schtasks")
            .args(["/query", "/tn", &config.name])
            .command_stdout()
            .is_some())
    }

    fn is_run(&self, config: &StartupConfig, _scope: Scope) -> Result<bool> {
        Ok(program_running(&config.program))
    }

    fn install(&self, config: &StartupConfig, scope: Scope) -> Result<()> {
        // /f 覆盖同名任务。
        let mut args = vec![
            "/create".to_string(),
            "/tn".to_string(),
            config.name.clone(),
            "/tr".to_string(),
            config.command_line(),
            "/sc".to_string(),
        ];

        match scope {
            // /sc onstart 开机启动；/ru SYSTEM 以系统身份运行，不依赖用户登录。
            Scope::Root => {
                args.push("onstart".to_string());
                args.push("/ru".to_string());
                args.push("SYSTEM".to_string());
                args.push("/rl".to_string());
                args.push("highest".to_string());
            }
            // /sc onlogon 登录时启动；不指定 /ru 时以当前登录用户身份运行，
            // 此时 /rl highest 要求管理员权限，受限账号会创建失败，故不加该参数。
            Scope::User => args.push("onlogon".to_string()),
        }

        args.push("/f".to_string());

        Command::new("schtasks").args(&args).command_run()
    }

    fn uninstall(&self, config: &StartupConfig, _scope: Scope) -> Result<()> {
        // /f 跳过删除确认，避免命令等待输入。
        Command::new("schtasks")
            .args(["/delete", "/tn", &config.name, "/f"])
            .command_run()
    }

    fn start(&self, config: &StartupConfig, _scope: Scope) -> Result<()> {
        Command::new("schtasks")
            .args(["/run", "/tn", &config.name])
            .command_run()
    }

    fn stop(&self, config: &StartupConfig, _scope: Scope) -> Result<()> {
        Command::new("schtasks")
            .args(["/end", "/tn", &config.name])
            .command_run()
    }
}

/// 目标可执行文件是否有进程在运行。
///
/// 按可执行文件的完整路径比对，因此手动启动同一路径的程序同样算作运行中，
/// 而仅仅同名、路径不同的程序不会被误判。路径比较前会做归一化，消除大小写与短路径差异。
fn program_running(program: &Path) -> bool {
    let Some(program) = normalize(program) else {
        return false;
    };

    // 只刷新进程与可执行文件路径，避免连带刷新内存与 CPU 信息。
    let system = System::new_with_specifics(
        RefreshKind::nothing()
            .with_processes(ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet)),
    );

    system.processes().values().any(|process| {
        process
            .exe()
            .and_then(normalize)
            .is_some_and(|candidate| candidate == program)
    })
}

/// 归一化路径，用于消除大小写、相对路径与短路径差异；路径不存在时返回 `None`。
fn normalize(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

//! Linux systemd 实现：写入单元文件，再用 `systemctl` 控制。
//!
//! 系统级单元位于 `/etc/systemd/system`，用户级单元位于 `~/.config/systemd/user`；
//! 用户级操作需要额外的 `--user` 参数，并要求该用户存在可用的 systemd 用户实例。

use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;
use framework_system::CommandExt;

use crate::config::StartupConfig;
use crate::directory::{self, SYSTEM_UNIT_DIRECTORY};
use crate::file;
use crate::platform::Platform;
use crate::scope::Scope;

/// systemd 单元文件后缀。
const UNIT_SUFFIX: &str = ".service";

/// systemd 自启实现。
pub(crate) struct LinuxStartup;

impl Platform for LinuxStartup {
    fn exists(&self, config: &StartupConfig, scope: Scope) -> Result<bool> {
        // 注册即在单元目录落文件，判断文件是否存在即可，无需 root 权限。
        Ok(unit_path(config, scope)?.exists())
    }

    fn is_run(&self, config: &StartupConfig, scope: Scope) -> Result<bool> {
        // is-active 在单元未运行时返回非零退出码，输出不是 running 即视为未运行。
        let output = Command::new("systemctl")
            .args(arguments(scope, &["is-active", &config.name]))
            .command_stdout();

        Ok(output.is_some_and(|output| output.trim() == "active"))
    }

    fn install(&self, config: &StartupConfig, scope: Scope) -> Result<()> {
        file::write_file(&unit_path(config, scope)?, &unit_content(config, scope))?;
        Command::new("systemctl")
            .args(arguments(scope, &["daemon-reload"]))
            .command_run()?;
        // 未启用的单元不会随系统启动，注册流程必须一并启用。
        Command::new("systemctl")
            .args(arguments(scope, &["enable", &config.name]))
            .command_run()
    }

    fn uninstall(&self, config: &StartupConfig, scope: Scope) -> Result<()> {
        // 未启用时 disable 会报错，注销流程不应因此中断。
        let _ = Command::new("systemctl")
            .args(arguments(scope, &["disable", &config.name]))
            .command_run();
        file::remove_file(&unit_path(config, scope)?)?;
        Command::new("systemctl")
            .args(arguments(scope, &["daemon-reload"]))
            .command_run()
    }

    fn start(&self, config: &StartupConfig, scope: Scope) -> Result<()> {
        Command::new("systemctl")
            .args(arguments(scope, &["start", &config.name]))
            .command_run()
    }

    fn stop(&self, config: &StartupConfig, scope: Scope) -> Result<()> {
        Command::new("systemctl")
            .args(arguments(scope, &["stop", &config.name]))
            .command_run()
    }
}

/// `systemctl` 参数；用户级单元需要 `--user`。
fn arguments(scope: Scope, args: &[&str]) -> Vec<String> {
    let mut arguments = match scope {
        Scope::Root => Vec::new(),
        Scope::User => vec!["--user".to_string()],
    };
    arguments.extend(args.iter().map(|arg| (*arg).to_string()));

    arguments
}

/// 单元文件路径。
fn unit_path(config: &StartupConfig, scope: Scope) -> Result<PathBuf> {
    let root = directory::directory(SYSTEM_UNIT_DIRECTORY, directory::user_unit_directory, scope)?;

    Ok(root.join(format!("{}{UNIT_SUFFIX}", config.name)))
}

/// 单元文件内容；工作目录仅在配置给出时写入。
///
/// `[Install]` 段的目标随级别不同：系统级单元挂到 `multi-user.target`，
/// 用户实例中不存在该 target，用户级单元必须挂到 `default.target`。
fn unit_content(config: &StartupConfig, scope: Scope) -> String {
    let working_directory = config
        .working_directory
        .as_ref()
        .map(|path| format!("WorkingDirectory={}\n", path.display()))
        .unwrap_or_default();
    let wanted_by = match scope {
        Scope::Root => "multi-user.target",
        Scope::User => "default.target",
    };

    format!(
        "[Unit]\n\
         Description={description}\n\
         After=network-online.target\n\
         Wants=network-online.target\n\
         \n\
         [Service]\n\
         Type=simple\n\
         ExecStart={command}\n\
         {working_directory}\
         Restart=on-failure\n\
         RestartSec=5\n\
         \n\
         [Install]\n\
         WantedBy={wanted_by}\n",
        description = config.description,
        command = config.command_line(),
    )
}

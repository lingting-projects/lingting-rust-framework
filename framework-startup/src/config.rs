use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};

/// 开机自启的注册配置；三个操作系统共用同一份配置。
#[derive(Debug, Clone)]
pub struct StartupConfig {
    /// 自启名称，同时作为 Windows 计划任务名、systemd 单元名与 launchd 标签。
    pub name: String,
    /// 自启描述。
    pub description: String,
    /// 要启动的可执行文件绝对路径；为空表示当前运行中的可执行文件。
    pub program: PathBuf,
    /// 启动参数。
    pub args: Vec<String>,
    /// 工作目录；为空表示由系统决定。
    pub working_directory: Option<PathBuf>,
    /// 自启未注册时 `start`、`stop`、`is_run`、`restart` 与 `uninstall` 的行为：
    /// 为真时直接返回成功或假，为假时返回错误。默认为真，便于把这些操作当作清理手段重复调用。
    pub ignore_not_installed: bool,
}

impl StartupConfig {
    /// 创建配置；启动目标默认为当前运行中的可执行文件，未注册时忽略。
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            program: PathBuf::new(),
            args: Vec::new(),
            working_directory: None,
            ignore_not_installed: true,
        }
    }

    /// 补全启动目标：未指定可执行文件时取当前运行中的可执行文件。
    pub(crate) fn resolve(mut self) -> Result<Self> {
        if self.program.as_os_str().is_empty() {
            self.program = env::current_exe().context("获取当前可执行文件路径失败")?;
        }

        Ok(self)
    }

    /// 完整的启动命令行：可执行文件绝对路径加启动参数，含空格的路径加引号；
    /// 供 Windows 计划任务与 systemd 单元使用，launchd 需要拆成独立的参数元素。
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    pub(crate) fn command_line(&self) -> String {
        let mut line = quote(&self.program.to_string_lossy());
        for arg in &self.args {
            line.push(' ');
            line.push_str(&quote(arg));
        }

        line
    }
}

/// 命令行参数加引号；参数本身含双引号时原样返回，避免破坏用户输入。
#[cfg(any(target_os = "windows", target_os = "linux"))]
fn quote(value: &str) -> String {
    if value.contains('"') || !value.contains(' ') {
        return value.to_string();
    }

    format!("\"{value}\"")
}

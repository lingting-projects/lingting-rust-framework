//! macOS launchd 实现：写入 plist，再用 `launchctl` 控制。
//!
//! 系统级 plist 位于 `/Library/LaunchDaemons`，用户级 plist 位于 `~/Library/LaunchAgents`；
//! 两者的 `launchctl` 命令形式一致，由系统按 plist 所在目录决定运行身份。

use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;
use framework_system::CommandExt;

use crate::config::StartupConfig;
use crate::directory::{self, SYSTEM_PLIST_DIRECTORY};
use crate::file;
use crate::platform::Platform;
use crate::scope::Scope;

/// launchd 配置文件后缀。
const PLIST_SUFFIX: &str = ".plist";

/// launchd 自启实现。
pub(crate) struct MacosStartup;

impl Platform for MacosStartup {
    fn exists(&self, config: &StartupConfig, scope: Scope) -> Result<bool> {
        // 注册即在对应目录落 plist，判断文件是否存在即可，无需 root 权限。
        Ok(plist_path(config, scope)?.exists())
    }

    fn is_run(&self, config: &StartupConfig, _scope: Scope) -> Result<bool> {
        // list 输出中带 PID 键表示正在运行；作业未加载时返回非零退出码，取不到输出即视为未运行。
        let output = Command::new("launchctl")
            .args(["list", &config.name])
            .command_stdout();

        Ok(output.is_some_and(|output| output.contains("\"PID\"")))
    }

    fn install(&self, config: &StartupConfig, scope: Scope) -> Result<()> {
        let plist = plist_path(config, scope)?;
        let plist_text = plist.to_string_lossy().to_string();

        file::write_file(&plist, &plist_content(config))?;
        // -w 覆盖已有的 disabled 标记，保证随系统启动。
        Command::new("launchctl")
            .args(["load", "-w", plist_text.as_str()])
            .command_run()
    }

    fn uninstall(&self, config: &StartupConfig, scope: Scope) -> Result<()> {
        // 未加载时 remove 会报错，注销流程不应因此中断。
        let _ = Command::new("launchctl")
            .args(["remove", &config.name])
            .command_run();
        file::remove_file(&plist_path(config, scope)?)
    }

    fn start(&self, config: &StartupConfig, _scope: Scope) -> Result<()> {
        Command::new("launchctl")
            .args(["start", &config.name])
            .command_run()
    }

    fn stop(&self, config: &StartupConfig, _scope: Scope) -> Result<()> {
        Command::new("launchctl")
            .args(["stop", &config.name])
            .command_run()
    }
}

/// plist 文件路径。
fn plist_path(config: &StartupConfig, scope: Scope) -> Result<PathBuf> {
    let root = directory::directory(
        SYSTEM_PLIST_DIRECTORY,
        directory::user_plist_directory,
        scope,
    )?;

    Ok(root.join(format!("{}{PLIST_SUFFIX}", config.name)))
}

/// plist 文件内容；工作目录仅在配置给出时写入。
fn plist_content(config: &StartupConfig) -> String {
    let working_directory = config
        .working_directory
        .as_ref()
        .map(|path| {
            format!(
                "\t<key>WorkingDirectory</key>\n\t<string>{}</string>\n",
                escape(&path.to_string_lossy())
            )
        })
        .unwrap_or_default();

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \t<key>Label</key>\n\
         \t<string>{label}</string>\n\
         \t<key>ProgramArguments</key>\n\
         \t<array>\n\
         {arguments}\
         \t</array>\n\
         {working_directory}\
         \t<key>RunAtLoad</key>\n\
         \t<true/>\n\
         </dict>\n\
         </plist>\n",
        label = escape(&config.name),
        arguments = arguments(config),
    )
}

/// `ProgramArguments` 数组内容：可执行文件绝对路径加启动参数。
fn arguments(config: &StartupConfig) -> String {
    let mut arguments = format!(
        "\t\t<string>{}</string>\n",
        escape(&config.program.to_string_lossy())
    );
    for arg in &config.args {
        arguments.push_str(&format!("\t\t<string>{}</string>\n", escape(arg)));
    }

    arguments
}

/// XML 文本转义，避免路径中的 `&`、`<` 等字符破坏 plist 结构。
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

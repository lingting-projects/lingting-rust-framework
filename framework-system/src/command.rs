//! `std::process::Command` 的扩展。
//!
//! 方法统一使用 `command_` 前缀。执行失败的错误信息带上命令行、退出码与完整输出，
//! 便于直接展示给用户；探测类方法把「无法执行」与「未成功退出」统一按失败处理。

use std::process::{Child, Command, ExitStatus, Output};

use anyhow::{Context, Result, anyhow};

/// `std::process::Command` 的扩展。
///
/// [`CommandExt::command_run`]、[`CommandExt::command_output`]、[`CommandExt::command_status`]
/// 与 [`CommandExt::command_spawn`] 只在命令无法执行时报错，退出码由调用方判断；
/// [`CommandExt::command_success`]、[`CommandExt::command_stdout`] 与 [`CommandExt::command_stderr`]
/// 则把「无法执行」与「未成功退出」统一按失败处理，用于探测类调用。
pub trait CommandExt {
    /// 执行命令并返回是否成功；命令无法执行或退出码非零时为假，不向上报错。
    fn command_success(&mut self) -> bool;

    /// 执行命令并检查退出码；失败时把命令行、退出码与完整输出一起报错。
    fn command_run(&mut self) -> Result<()>;

    /// 执行命令并返回完整输出；仅命令无法执行时报错。
    fn command_output(&mut self) -> Result<Output>;

    /// 执行命令并返回退出状态；仅命令无法执行时报错。
    fn command_status(&mut self) -> Result<ExitStatus>;

    /// 执行命令并返回标准输出；命令无法执行或退出码非零时返回 `None`。
    ///
    /// 用于状态探测：探测失败与「探测结果为否」由调用方统一按否处理，不作为错误向上传播。
    fn command_stdout(&mut self) -> Option<String>;

    /// 执行命令并返回标准错误；命令无法执行或退出码非零时返回 `None`。
    fn command_stderr(&mut self) -> Option<String>;

    /// 启动命令并立即返回子进程句柄，不等待其结束。
    fn command_spawn(&mut self) -> Result<Child>;

    /// 命令行文本：程序与参数以空格连接，用于展示与错误信息。
    fn command_line(&self) -> String;
}

impl CommandExt for Command {
    fn command_success(&mut self) -> bool {
        self.output().is_ok_and(|output| output.status.success())
    }

    fn command_run(&mut self) -> Result<()> {
        let output = self.command_output()?;
        if output.status.success() {
            return Ok(());
        }

        Err(failure(self, &output))
    }

    fn command_output(&mut self) -> Result<Output> {
        let program = program(self);

        self.output()
            .with_context(|| format!("执行 {program} 失败，请确认该命令存在且可用"))
    }

    fn command_status(&mut self) -> Result<ExitStatus> {
        let program = program(self);

        self.status()
            .with_context(|| format!("执行 {program} 失败，请确认该命令存在且可用"))
    }

    fn command_stdout(&mut self) -> Option<String> {
        Some(text(&self.command_output().ok()?.stdout))
    }

    fn command_stderr(&mut self) -> Option<String> {
        Some(text(&self.command_output().ok()?.stderr))
    }

    fn command_spawn(&mut self) -> Result<Child> {
        let program = program(self);

        self.spawn()
            .with_context(|| format!("启动 {program} 失败，请确认该命令存在且可用"))
    }

    fn command_line(&self) -> String {
        let mut line = program(self);
        for arg in self.get_args() {
            line.push(' ');
            line.push_str(&arg.to_string_lossy());
        }

        line
    }
}

/// 命令的程序名。
fn program(command: &Command) -> String {
    command.get_program().to_string_lossy().into_owned()
}

/// 退出码非零的错误：带上命令行、退出码与完整输出。
fn failure(command: &Command, output: &Output) -> anyhow::Error {
    let code = output
        .status
        .code()
        .map_or_else(|| "未知".to_string(), |code| code.to_string());

    anyhow!(
        "{} 执行失败\n  命令：{}\n  退出码：{code}\n  标准输出：{}\n  标准错误：{}",
        program(command),
        command.command_line(),
        output_text(&output.stdout),
        output_text(&output.stderr),
    )
}

/// 外部命令输出转文本。
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// 外部命令输出转文本；空输出用占位符表示，避免错误信息里出现空白。
fn output_text(bytes: &[u8]) -> String {
    let text = text(bytes);
    let text = text.trim();
    if text.is_empty() {
        "(空)".to_string()
    } else {
        text.to_string()
    }
}

//! 执行外部命令。

use std::ffi::OsStr;
use std::process::Command;

use anyhow::{Context, Result, anyhow};

/// 执行外部命令；失败时把命令、退出码与完整输出一起报错，便于直接展示给用户。
pub(crate) fn run<S: AsRef<OsStr>>(program: &str, args: &[S]) -> Result<()> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("执行 {program} 失败，请确认该命令存在且可用"))?;

    if output.status.success() {
        return Ok(());
    }

    let code = output
        .status
        .code()
        .map_or_else(|| "未知".to_string(), |code| code.to_string());

    Err(anyhow!(
        "{program} 执行失败\n  命令：{program} {}\n  退出码：{code}\n  标准输出：{}\n  标准错误：{}",
        join(args),
        output_text(&output.stdout),
        output_text(&output.stderr),
    ))
}

/// 执行外部命令并返回标准输出；命令无法执行或退出码非零时返回 `None`。
///
/// 用于状态探测：探测失败与「探测结果为否」由调用方统一按否处理，不作为错误向上传播。
pub(crate) fn stdout<S: AsRef<OsStr>>(program: &str, args: &[S]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// 参数转成可直接展示的命令行文本。
fn join<S: AsRef<OsStr>>(args: &[S]) -> String {
    args.iter()
        .map(|arg| arg.as_ref().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// 外部命令输出转文本；空输出用占位符表示，避免错误信息里出现空白。
fn output_text(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim();
    if text.is_empty() {
        "(空)".to_string()
    } else {
        text.to_string()
    }
}

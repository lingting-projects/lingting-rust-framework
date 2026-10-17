# framework-system

提供操作系统相关能力：提权判断与外部命令扩展，Windows、Linux、macOS 使用同一套 API。

## 平台支持

| 能力 | Windows | Linux | macOS |
|------|---------|-------|-------|
| `is_elevated()` | 查询进程令牌的 `TokenElevation` | `geteuid() == 0` | `geteuid() == 0` |
| `CommandExt` | 通用 | 通用 | 通用 |

各平台实现按编译目标门控，非目标平台的代码与依赖不参与编译：Windows 依赖 `windows-sys`，
Linux 与 macOS 依赖 `libc`，其余平台不引入任何平台依赖。

## is_elevated

判断当前进程是否以管理员（Windows）或 root（Linux、macOS）身份运行。无法判断的平台始终返回假。

```rust
use framework_system::is_elevated;

if is_elevated()? {
    println!("当前已提权");
}
# Ok::<(), anyhow::Error>(())
```

## CommandExt

`std::process::Command` 的扩展，方法统一使用 `command_` 前缀。

| 方法 | 返回 | 说明 |
|------|------|------|
| `command_success()` | `bool` | 执行命令并返回是否成功；无法执行或退出码非零均为假，不报错 |
| `command_run()` | `Result<()>` | 执行命令并检查退出码；失败时带上命令行、退出码与完整输出报错 |
| `command_output()` | `Result<Output>` | 执行命令并返回完整输出；仅无法执行时报错 |
| `command_status()` | `Result<ExitStatus>` | 执行命令并返回退出状态；仅无法执行时报错 |
| `command_stdout()` | `Option<String>` | 执行命令并返回标准输出；无法执行或退出码非零时返回 `None` |
| `command_stderr()` | `Option<String>` | 执行命令并返回标准错误；无法执行或退出码非零时返回 `None` |
| `command_spawn()` | `Result<Child>` | 启动命令并立即返回子进程句柄，不等待其结束 |
| `command_line()` | `String` | 命令行文本：程序与参数以空格连接，用于展示与错误信息 |

错误处理分为两类：

- `command_run`、`command_output`、`command_status`、`command_spawn` 只在命令无法执行时报错，
  退出码由调用方判断。
- `command_success`、`command_stdout`、`command_stderr` 把「无法执行」与「未成功退出」统一按失败处理，
  用于状态探测，探测失败与「探测结果为否」由调用方统一按否处理。

```rust
use std::process::Command;

use framework_system::CommandExt;

// 失败时错误信息包含命令行、退出码、标准输出与标准错误，可直接展示给用户
Command::new("systemctl").args(["--user", "start", "my-app"]).command_run()?;

// 状态探测：命令不存在或未成功退出都按「未运行」处理
let active = Command::new("systemctl")
    .args(["--user", "is-active", "my-app"])
    .command_stdout()
    .is_some_and(|output| output.trim() == "active");

// 后台启动
let mut child = Command::new("my-app").command_spawn()?;
let _ = child.wait();
# Ok::<(), anyhow::Error>(())
```

# framework-startup

提供开机自启的注册、注销与启停能力，Windows、Linux、macOS 使用同一套 API。

## 平台实现

| 级别 | Windows | Linux | macOS |
|------|---------|-------|-------|
| `Startup::root` | 计划任务 `onstart`，以 `SYSTEM` 身份运行 | `/etc/systemd/system/<name>.service` | `/Library/LaunchDaemons/<name>.plist` |
| `Startup::user` | 计划任务 `onlogon`，以当前用户身份运行 | `~/.config/systemd/user/<name>.service` | `~/Library/LaunchAgents/<name>.plist` |

系统级随系统启动，不依赖用户登录，注册与启停通常需要管理员或 root 权限；用户级随当前用户登录启动，
无需提权。Linux 用户级还要求该用户存在可用的 systemd 用户实例。

实现方式为「写入自启定义文件 + 调用系统命令」：Windows 调用 `schtasks`，Linux 调用 `systemctl`
（用户级附加 `--user`），macOS 调用 `launchctl`。命令失败时会带上退出码与完整输出，便于直接展示给用户。

外部命令通过 [`framework-system`](../framework-system/README.md) 的 `CommandExt` 执行，错误信息与命令行
由该扩展统一生成。`sysinfo` 仅在 Windows 目标下参与编译，用于按可执行文件路径判断运行状态。

## 用法

```rust
use framework_startup::{Startup, StartupConfig};

// 以当前可执行文件为启动目标，注册随当前用户登录启动
let mut config = StartupConfig::new("my-app");
config.description = "个人使用的轻量网关".to_string();
config.args = vec!["--port".to_string(), "8080".to_string()];
config.working_directory = Some("/opt/my-app".into());

let startup = Startup::user(config)?;

// install 已注册时不重复执行；需要让配置变更生效时用 install_force
startup.install()?;
// start 已在运行时跳过，stop 未运行时跳过
startup.start()?;
startup.restart()?;

# Ok::<(), anyhow::Error>(())
```

系统级自启把 `Startup::user` 换成 `Startup::root` 即可，其余调用完全一致。

## 配置

`StartupConfig` 的字段：

| 字段 | 说明 |
|------|------|
| `name` | 自启名称，同时作为计划任务名、systemd 单元名与 launchd 标签 |
| `description` | 自启描述，写入 systemd 单元的 `Description` |
| `program` | 要启动的可执行文件绝对路径；为空时取当前运行中的可执行文件 |
| `args` | 启动参数 |
| `working_directory` | 工作目录；为空表示由系统决定 |
| `ignore_not_installed` | 未注册时 `start` / `stop` / `is_run` / `restart` / `uninstall` 是忽略还是报错，默认为 `true` |

`StartupConfig::new(name)` 只设置名称，其余字段按需赋值。

## API

| 方法 | 说明 |
|------|------|
| `Startup::root(config)` | 创建系统级自启 |
| `Startup::user(config)` | 创建当前用户级自启 |
| `exists()` | 查询自启是否已注册 |
| `is_run()` | 查询自启的进程是否正在运行 |
| `install()` | 注册自启；已注册时不重复执行，直接返回成功 |
| `install_force()` | 注册自启并覆盖已存在的同名自启，用于让配置变更生效 |
| `uninstall()` | 注销自启，不停止正在运行的进程 |
| `start()` | 立即启动；已在运行时直接返回成功 |
| `stop()` | 立即停止；未运行时直接返回成功 |
| `restart()` | 重启：未运行时直接启动，正在运行时停止后再启动 |

`start()`、`stop()`、`is_run()`、`restart()` 与 `uninstall()` 只对已注册的自启执行；未注册时按
`StartupConfig::ignore_not_installed` 决定忽略（默认，`is_run()` 返回假）还是报错。
`start()` 与 `stop()` 还会先判断运行状态，因此重复调用不会重复触发底层命令。
`restart()` 只探测一次运行状态，需要重启时等价于一次 `stop()` 加一次 `start()`，但少一次探测。

`uninstall()` 与 `stop()` 相互独立：需要注销的同时结束进程时，先调 `stop()` 再调 `uninstall()`。


## 平台差异

- Windows 计划任务的启动目标是完整命令行；systemd 单元的 `ExecStart` 与 launchd 的
  `ProgramArguments` 由同一份配置生成，含空格的路径会自动加引号或拆分。
- Windows 计划任务名在任务树的根目录下唯一，系统级与用户级不能使用同名任务；systemd 与 launchd
  的两种级别存放于不同目录，可以同名。
- Linux 用户级单元的 `[Install]` 目标为 `default.target`（系统级为 `multi-user.target`）。
- launchd 的 `start` / `stop` 只作用于已加载的作业，`install` 会以 `launchctl load -w` 加载并
  清除 disabled 标记。
- `is_run()` 的判定方式：Linux 用 `systemctl is-active`，macOS 用 `launchctl list` 是否带 `PID` 键，
  Windows 按可执行文件完整路径比对进程（`sysinfo`），因此手动启动同一路径的程序同样算作运行中。

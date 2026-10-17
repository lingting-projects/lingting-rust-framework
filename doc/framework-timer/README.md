# framework-timer

基于 Tokio 的进程内定时任务运行时：`#[timer]` 宏把 async 函数注册到链接期全局注册表，
调度器按各自的间隔派发任务，单个任务的阻塞或 panic 不影响其他任务。

## 注册任务

```rust
use framework_timer::{TimerResult, timer};

#[timer(interval = 1_000, count = 2)]
async fn cleanup_expired_session() -> TimerResult {
    // 业务逻辑
    TimerResult::success()
}
```

任务名固定为函数名，因此同一进程内不能出现同名任务。

| 参数 | 说明 |
|------|------|
| `interval` | 必填，默认执行间隔（毫秒） |
| `count` | 同时运行的实例数，默认 `1`，多实例名称带 `_1`、`_2` 后缀 |
| `retry_immediately_on_panic` | 本任务 panic 后是否立即重试，覆盖 `TimerOption` 的同名配置 |

函数必须无参、async，且返回 `TimerResult`。`TimerResult::with_next` 可指定下次执行的毫秒时间戳，
`TimerResult::failure` 记录错误日志后仍按默认间隔重试。

## 启动运行时

```rust
use framework_timer::{TimerOption, start};

start(None, TimerOption {
    max_concurrent_tasks: Some(8),
    retry_immediately_on_panic: false,
});
```

`start` 只能调用一次。`wrapper` 用于为任务建立外部异步上下文，例如注入请求级 tracing span。

## 唤醒任务

`wake` 接受任何实现了 `TimerNameLike` 的类型，字符串、`String`、`Cow<str>`
以及生成的名称枚举都可以直接传入：

```rust
use framework_timer::wake;

wake("cleanup_expired_session");
wake(String::from("cleanup_expired_session"));
```

## 生成名称枚举

开启 `generate` feature 后可使用 `generate_name::generate` 扫描源码目录中的 `#[timer]` 任务，
返回完整的枚举源码，由调用方决定写入文件还是直接使用：

```toml
[dependencies]
framework-timer = { version = "26.8.150", features = ["generate"] }
```

```rust
use framework_timer::generate_name;

let code = generate_name::generate("src")?;
std::fs::write("src/timer_name.rs", code)?;

// 也可以自定义枚举名
let code = generate_name::generate_named("src", "MyTimerName")?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

生成的枚举提供 `as_str`，实现 `TimerNameLike` 与 `Display`，可与字符串混用：

```rust
use framework_timer::wake;
use crate::timer_name::TimerName;

wake(TimerName::CleanupExpiredSession);
```

任务重名或变体名冲突时 `generate` 返回错误。该 feature 默认关闭，因此运行时不额外依赖 `syn`。

## 直接提交注册

不使用宏时可通过 `push_timer` 手工提交 `TimerRegistration`，`timer_iter` 用于遍历当前进程的
全部注册项。

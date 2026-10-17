# framework-proc-timer

`#[timer]` 属性宏，把 async 函数注册为 [`framework-timer`](../framework-timer/README.md) 的定时任务。

## 用法

```rust
use framework_timer::{TimerResult, timer};

#[timer(interval = 60_000)]
async fn refresh_cache() -> TimerResult {
    TimerResult::success()
}
```

## 参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `interval` | 是 | 默认执行间隔（毫秒） |
| `count` | 否 | 同时运行的实例数，默认 `1`，必须大于 0 |
| `retry_immediately_on_panic` | 否 | 本任务 panic 后是否立即重试，覆盖运行时配置 |

参数必须使用 `key = value` 形式，且不能重复指定。

## 约束

- 只能标注 async 函数；
- 函数不能包含参数；
- 返回值必须是 `framework_timer::TimerResult`；
- 任务名固定为函数名，同一进程内不能出现同名任务。

宏展开后生成一个目标函数包装，并通过 `framework_timer::inventory::submit!` 提交
`TimerRegistration`，因此使用方无需额外启用 `inventory`。

# framework-bus

与 UI 框架完全无关的进程内通知总线。业务 crate 发出“某类事情发生了”的通知，关心的模块订阅该类通知
并自行读取最新状态；业务 crate 只依赖本 crate，不会间接依赖任何 UI 框架。

典型场景：安装任务完成 → 发出 `InstallFinished` 通知 → 任务页、控制台页各自刷新数据。

## 设计

- **纯通知**：通知不携带数据，订阅方被唤醒后自己去读最新状态。
- **类型即事件**：每种事件是一个独立的 Rust 类型（通常是空结构体），总线内部以 `TypeId` 区分。
  总线不认识任何业务事件，各业务域把事件类型定义在自己的 types crate 中，订阅方只依赖它关心的那个类型。
- **不存储事件记录**：没有事件队列、没有历史、没有回放。
- **异步由注入的分发者提供**：总线本身不起线程、不依赖任何 runtime，处理函数在哪里、以什么方式执行
  由外部注入的 `Dispatcher` 决定。
- **发布永远成功**：`notify` 无错误返回。总线关闭后同样视为成功，并输出警告日志。
- **边沿触发 + 合并**：订阅方收到的是“自上次处理以来至少发生过一次”，不保证次数。

依赖只有 `log` 与仅开 `sync` feature 的 `tokio`（只用 `Notify`），不依赖 gpui。

## 使用

```rust
use framework_bus::Bus;

// 事件类型：放在业务域自己的 types crate，无需依赖 framework-bus
pub struct InstallFinished;
pub struct TaskChanged;

// 启动时
let bus = Bus::new(); // 或 Bus::with_dispatcher(my_dispatcher)

// 业务模块：发出通知
bus.notify::<InstallFinished>();

// 业务模块：订阅（由 Dispatcher 执行）
let sub = bus.subscribe::<InstallFinished>(|| refresh_cache());

// 消费扩展：异步等待（合并）
let mut w = bus.watch::<TaskChanged>();
while w.changed().await.is_ok() {
    reload().await;
}
# fn refresh_cache() {}
# async fn reload() {}
# drop((sub, w));
```

### 全局单例

库内不强制单例，需要时由使用方自行定义：

```rust
use framework_bus::Bus;
use std::sync::OnceLock;

static BUS: OnceLock<Bus> = OnceLock::new();

pub fn bus() -> &'static Bus {
    BUS.get_or_init(Bus::new)
}
```

### 自定义 Dispatcher

`Dispatcher` 决定处理函数在哪里、何时执行。例如投递到 tokio 的阻塞线程池：

```rust
use framework_bus::{Dispatcher, Job};

struct TokioDispatcher(tokio::runtime::Handle);

impl Dispatcher for TokioDispatcher {
    fn dispatch(&self, job: Job) {
        self.0.spawn_blocking(move || job.run());
    }
}
```

实现约定：`dispatch` 不应长时间阻塞调用者，任务应当最终被执行或被丢弃（丢弃是允许的，
总线的合并标记会自动复位）。

## API

| 项目 | 说明 |
|------|------|
| `Bus::new()` | 使用 `InlineDispatcher`，在发布者线程内同步执行处理函数 |
| `Bus::with_dispatcher()` | 注入自定义分发者 |
| `Bus::set_dispatcher()` | 运行期替换分发者，之后的通知使用新分发者 |
| `Bus::notify::<E>()` | 发出 `E` 类通知，任意线程可调用、无需 runtime、永不失败、永不阻塞 |
| `Bus::subscribe::<E>()` | 订阅 `E` 类通知，返回的 `Subscription` drop 即退订 |
| `Bus::watch::<E>()` | 得到一个可 await 的合并通知句柄，不经过 `Dispatcher` |
| `Bus::close()` / `is_closed()` | 关闭总线（幂等）与查询关闭状态 |
| `Bus::subscriber_count::<E>()` | `E` 类通知的订阅数，含 `subscribe` 与 `watch`，便于测试与诊断 |
| `Subscription::detach()` | 放弃句柄，订阅一直保留到总线关闭 |
| `Watcher::changed()` | 等待“自上次返回以来至少发生过一次通知”，关闭后返回 `Err(Closed)` |

API 中没有事件枚举，通知也不携带任何业务数据；不提供按条件过滤订阅，需要区分情形时拆成不同的事件类型。

## 行为约定

1. 通知**不携带数据**。收到后应读取最新状态；处理函数必须**幂等**，不得依赖被调用次数。
2. 通知是**边沿触发、不回放**：订阅之前发出的通知收不到。使用方应**先订阅，再加载当前状态**，
   避免二者之间的变化被漏掉。
3. 同一订阅的处理函数调用会被**合并**：在任务开始执行之前到达的多次通知只触发一次；
   执行期间到达的通知会再触发一次。
4. 不同订阅之间互不影响；同一订阅的处理函数不会被并发执行两次的保证**由 Dispatcher 决定**
   （`InlineDispatcher` 下可能在不同线程并发进入，处理函数需自行保证线程安全；如需串行，
   使用串行的 Dispatcher 实现）。
5. `Watcher` 不经过 Dispatcher，在 `notify` 调用线程内被唤醒，开销极小。
6. 处理函数应轻量、非阻塞；耗时工作应在处理函数中自行投递出去。
7. 总线关闭后 `notify` 只打警告；订阅与 `watch` 得到无效句柄。

最后一个 `Bus` 克隆被释放时，共享状态随之销毁，等价于关闭：等待中的 `Watcher::changed()`
返回 `Err(Closed)`。

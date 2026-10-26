//! 与 UI 框架完全无关的进程内通知总线。
//!
//! 业务 crate 发出“某类事情发生了”的通知，关心的模块订阅该类通知并自行读取最新状态。
//! 业务 crate 只依赖本 crate，不会间接依赖任何 UI 框架。
//!
//! # 设计
//!
//! - **纯通知**：通知不携带数据，订阅方被唤醒后自己去读最新状态。
//! - **类型即事件**：每种事件是一个独立的 Rust 类型（通常是空结构体），总线内部以 [`TypeId`] 区分。
//!   各业务域把事件类型定义在自己的 types crate 中，订阅方只依赖它关心的那个类型。
//! - **不存储事件记录**：没有事件队列、没有历史、没有回放。总线内部只保存订阅关系。
//! - **异步由注入的分发者提供**：总线本身不起线程、不依赖任何 runtime，处理函数在哪里、
//!   以什么方式执行由 [`Dispatcher`] 决定。
//! - **发布永远成功**：[`Bus::notify`] 无错误返回，总线关闭后同样视为成功并输出警告日志。
//! - **边沿触发 + 合并**：订阅方收到的是“自上次处理以来至少发生过一次”，不保证次数。
//!
//! [`TypeId`]: std::any::TypeId
//!
//! # 使用
//!
//! ```rust
//! use framework_bus::Bus;
//!
//! // 事件类型：放在业务域自己的 types crate，无需依赖 framework-bus
//! pub struct InstallFinished;
//! pub struct TaskChanged;
//!
//! // 启动时
//! let bus = Bus::new();
//!
//! // 业务模块：订阅（由 Dispatcher 执行）
//! let sub = bus.subscribe::<InstallFinished>(|| {
//!     // 读取最新状态
//! });
//!
//! // 业务模块：发出通知
//! bus.notify::<InstallFinished>();
//! # drop(sub);
//! ```
//!
//! # 行为约定
//!
//! 1. 通知**不携带数据**。收到后应读取最新状态；处理函数必须**幂等**，不得依赖被调用次数。
//! 2. 通知是**边沿触发、不回放**：订阅之前发出的通知收不到。使用方应**先订阅，再加载当前状态**，
//!    避免二者之间的变化被漏掉。
//! 3. 同一订阅的处理函数调用会被**合并**：在任务开始执行之前到达的多次通知只触发一次；
//!    执行期间到达的通知会再触发一次。
//! 4. 不同订阅之间互不影响；同一订阅的处理函数不会被并发执行两次的保证**由 Dispatcher 决定**
//!    （[`InlineDispatcher`] 下可能在不同线程并发进入，处理函数需自行保证线程安全；
//!    如需串行，使用串行的 Dispatcher 实现）。
//! 5. [`Watcher`] 不经过 Dispatcher，在 [`Bus::notify`] 调用线程内被唤醒，开销极小。
//! 6. 处理函数应轻量、非阻塞；耗时工作应在处理函数中自行投递出去。
//! 7. 总线关闭后 [`Bus::notify`] 只打警告；订阅与 [`Bus::watch`] 得到无效句柄。

mod bus;
mod closed;
mod dispatcher;
mod inner;
mod subscription;

pub use bus::Bus;
pub use closed::Closed;
pub use dispatcher::{Dispatcher, InlineDispatcher, Job};
pub use subscription::{Subscription, Watcher};

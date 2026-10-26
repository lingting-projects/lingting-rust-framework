use crate::Dispatcher;
use crate::dispatcher::{InlineDispatcher, Job};
use crate::inner::{Entry, Inner, Kind};
use crate::subscription::{Subscription, Watcher};
use std::any::{TypeId, type_name};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 进程内通知总线。
///
/// `Clone` 为浅克隆，所有克隆共享同一份订阅表，全应用通常只有一个实例。
/// 总线只保存订阅关系，不保存任何通知记录；通知本身不携带数据。
#[derive(Clone)]
pub struct Bus {
    inner: Arc<Inner>,
}

impl Bus {
    /// 创建总线，使用 [`InlineDispatcher`] 在发布者线程内同步执行处理函数。
    pub fn new() -> Self {
        Self::with_dispatcher(Arc::new(InlineDispatcher))
    }

    /// 使用指定的分发者创建总线。
    pub fn with_dispatcher(dispatcher: Arc<dyn Dispatcher>) -> Self {
        Self {
            inner: Arc::new(Inner::new(dispatcher)),
        }
    }

    /// 运行期替换分发者，之后的通知使用新分发者。
    ///
    /// 已提交但尚未执行的任务不受影响。
    pub fn set_dispatcher(&self, dispatcher: Arc<dyn Dispatcher>) {
        self.inner.set_dispatcher(dispatcher);
    }

    /// 发出 `E` 类通知。
    ///
    /// 任意线程可调用，无需 runtime，永不失败，永不阻塞。总线已关闭时忽略本次通知并输出
    /// `warn` 日志，仍视为成功；无订阅者时静默成功。
    pub fn notify<E: 'static>(&self) {
        if self.inner.is_closed() {
            log::warn!(
                "framework-bus: notify({}) on closed bus, ignored",
                type_name::<E>()
            );
            return;
        }

        let entries = self.inner.snapshot(TypeId::of::<E>());

        if entries.is_empty() {
            return;
        }

        let dispatcher = self.inner.dispatcher();

        // 快照已脱离锁，处理函数里再订阅、再通知、再退订都不会死锁
        for entry in entries {
            match &entry.kind {
                // 唤醒等待者，Notify 最多保留一个许可，天然合并
                Kind::Watch { notify, .. } => notify.notify_one(),
                Kind::Handler { pending, .. } => {
                    // 已有任务在排队或正在执行，本次通知被合并
                    if pending.swap(true, Ordering::AcqRel) {
                        continue;
                    }

                    dispatcher.dispatch(handler_job(&entry));
                }
            }
        }
    }

    /// 订阅 `E` 类通知，处理函数由 [`Dispatcher`] 执行。
    ///
    /// 返回的 [`Subscription`] 被 drop 时自动退订。总线已关闭时不建立订阅，
    /// 返回的句柄为空操作。
    pub fn subscribe<E: 'static>(
        &self,
        handler: impl Fn() + Send + Sync + 'static,
    ) -> Subscription {
        let type_id = TypeId::of::<E>();
        let id = self.inner.next_id();
        let entry = Arc::new(Entry {
            id,
            name: type_name::<E>(),
            kind: Kind::Handler {
                f: Arc::new(handler),
                pending: AtomicBool::new(false),
            },
        });

        if !self.inner.insert(type_id, entry) {
            log::warn!(
                "framework-bus: subscribe({}) on closed bus, ignored",
                type_name::<E>()
            );
            return Subscription::inactive();
        }

        Subscription::new(Arc::downgrade(&self.inner), type_id, id)
    }

    /// 订阅 `E` 类通知并得到一个可 await 的合并通知句柄。
    ///
    /// 该句柄不经过 [`Dispatcher`]，在 [`notify`](Self::notify) 调用线程内被唤醒。
    pub fn watch<E: 'static>(&self) -> Watcher {
        let type_id = TypeId::of::<E>();
        let id = self.inner.next_id();
        let entry = Arc::new(Entry {
            id,
            name: type_name::<E>(),
            kind: Kind::Watch {
                notify: tokio::sync::Notify::new(),
                closed: AtomicBool::new(false),
            },
        });

        if !self.inner.insert(type_id, Arc::clone(&entry)) {
            log::warn!(
                "framework-bus: watch({}) on closed bus, ignored",
                type_name::<E>()
            );
            return Watcher::inactive();
        }

        Watcher::new(Arc::downgrade(&self.inner), type_id, entry)
    }

    /// 关闭总线，幂等。
    ///
    /// 之后的通知被忽略并输出 `warn` 日志，等待中的 [`Watcher::changed`] 返回
    /// [`Closed`](crate::Closed)，新订阅得到无效句柄。
    pub fn close(&self) {
        self.inner.close();
    }

    /// 总线是否已关闭。
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    /// `E` 类通知的订阅数，包含 [`subscribe`](Self::subscribe) 与 [`watch`](Self::watch)。
    pub fn subscriber_count<E: 'static>(&self) -> usize {
        self.inner.count(TypeId::of::<E>())
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

/// 为一条 `Handler` 订阅构造任务。
///
/// 任务执行时先复位合并标记再调用处理函数，因此处理函数执行期间到达的通知会触发新一轮；
/// 任务被分发者丢弃时由 [`Job`] 的丢弃守卫复位合并标记，避免该订阅永远不再触发。
fn handler_job(entry: &Arc<Entry>) -> Job {
    let run_entry = Arc::clone(entry);
    let reset_entry = Arc::clone(entry);
    let name = entry.name;

    let task = Box::new(move || {
        run_entry.clear_pending();

        let Kind::Handler { f, .. } = &run_entry.kind else {
            return;
        };

        if catch_unwind(AssertUnwindSafe(|| f())).is_err() {
            log::error!("framework-bus: handler of {} panicked", name);
        }
    });

    let reset = Box::new(move || reset_entry.clear_pending());

    Job::new(task, reset)
}

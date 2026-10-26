use crate::Dispatcher;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::sync::Notify;

/// 总线共享状态，`Bus` 的每个克隆都指向同一个 `Inner`。
pub(crate) struct Inner {
    state: RwLock<State>,
    dispatcher: RwLock<Arc<dyn Dispatcher>>,
    closed: AtomicBool,
    next_id: AtomicU64,
}

struct State {
    map: HashMap<TypeId, Vec<Arc<Entry>>>,
}

/// 一条订阅关系。
pub(crate) struct Entry {
    pub(crate) id: u64,
    /// 事件类型名，仅用于日志。
    pub(crate) name: &'static str,
    pub(crate) kind: Kind,
}

/// 订阅的两种形态：经分发者执行的处理函数，或直接唤醒的异步等待者。
pub(crate) enum Kind {
    Handler {
        f: Arc<dyn Fn() + Send + Sync>,
        /// 是否已有一个尚未开始执行的任务，用于合并通知。
        pending: AtomicBool,
    },
    Watch {
        notify: Notify,
        closed: AtomicBool,
    },
}

impl Entry {
    /// 清除合并标记，使下一次通知可以产生新任务。
    pub(crate) fn clear_pending(&self) {
        if let Kind::Handler { pending, .. } = &self.kind {
            pending.store(false, Ordering::Release);
        }
    }
}

impl Inner {
    pub(crate) fn new(dispatcher: Arc<dyn Dispatcher>) -> Self {
        Self {
            state: RwLock::new(State {
                map: HashMap::new(),
            }),
            dispatcher: RwLock::new(dispatcher),
            closed: AtomicBool::new(false),
            next_id: AtomicU64::new(0),
        }
    }

    /// 分配一个全局唯一的订阅 id。
    pub(crate) fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    /// 插入订阅。总线已关闭时不插入并返回 `false`。
    pub(crate) fn insert(&self, type_id: TypeId, entry: Arc<Entry>) -> bool {
        let mut state = write(&self.state);

        // 与 close 争用同一把写锁，保证关闭之后不再出现新订阅
        if self.is_closed() {
            return false;
        }

        state.map.entry(type_id).or_default().push(entry);
        true
    }

    /// 按 id 移除订阅，事件类型下没有订阅时一并清理。
    pub(crate) fn remove(&self, type_id: TypeId, id: u64) {
        let mut state = write(&self.state);

        let empty = match state.map.get_mut(&type_id) {
            Some(entries) => {
                entries.retain(|entry| entry.id != id);
                entries.is_empty()
            }
            None => false,
        };

        if empty {
            state.map.remove(&type_id);
        }
    }

    /// 取出某事件类型当前的订阅快照，调用方在锁外使用。
    pub(crate) fn snapshot(&self, type_id: TypeId) -> Vec<Arc<Entry>> {
        let state = read(&self.state);
        state.map.get(&type_id).cloned().unwrap_or_default()
    }

    pub(crate) fn count(&self, type_id: TypeId) -> usize {
        let state = read(&self.state);
        state.map.get(&type_id).map_or(0, Vec::len)
    }

    pub(crate) fn dispatcher(&self) -> Arc<dyn Dispatcher> {
        read(&self.dispatcher).clone()
    }

    pub(crate) fn set_dispatcher(&self, dispatcher: Arc<dyn Dispatcher>) {
        *write(&self.dispatcher) = dispatcher;
    }

    /// 关闭总线：清空订阅表并唤醒所有等待中的 `Watcher`。
    pub(crate) fn close(&self) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }

        let entries = drain(&mut write(&self.state));
        wake(entries);
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::Release);

        let entries = drain(self.state.get_mut().unwrap_or_else(|e| e.into_inner()));
        wake(entries);
    }
}

fn drain(state: &mut State) -> Vec<Arc<Entry>> {
    state.map.drain().flat_map(|(_, entries)| entries).collect()
}

/// 唤醒并标记所有 `Watch` 订阅，使等待中的 `changed()` 返回 `Err(Closed)`。
fn wake(entries: impl IntoIterator<Item = Arc<Entry>>) {
    for entry in entries {
        if let Kind::Watch { notify, closed } = &entry.kind {
            closed.store(true, Ordering::Release);
            notify.notify_one();
        }
    }
}

fn read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|e| e.into_inner())
}

fn write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|e| e.into_inner())
}

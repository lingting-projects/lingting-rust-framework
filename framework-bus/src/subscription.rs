use crate::Closed;
use crate::inner::{Entry, Inner, Kind};
use std::any::TypeId;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Weak};

/// 订阅句柄，drop 即退订。
#[must_use = "dropping a Subscription unsubscribes it"]
pub struct Subscription {
    inner: Weak<Inner>,
    type_id: TypeId,
    id: u64,
    active: bool,
}

impl Subscription {
    pub(crate) fn new(inner: Weak<Inner>, type_id: TypeId, id: u64) -> Self {
        Self {
            inner,
            type_id,
            id,
            active: true,
        }
    }

    /// 总线已关闭时得到的空句柄，drop 无任何效果。
    pub(crate) fn inactive() -> Self {
        Self {
            inner: Weak::new(),
            type_id: TypeId::of::<()>(),
            id: 0,
            active: false,
        }
    }

    /// 放弃句柄，订阅一直保留到总线关闭。
    pub fn detach(mut self) {
        self.active = false;
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if !self.active {
            return;
        }

        if let Some(inner) = self.inner.upgrade() {
            inner.remove(self.type_id, self.id);
        }
    }
}

/// 合并式异步通知句柄，持有期间保持订阅，drop 即退订。
pub struct Watcher {
    inner: Weak<Inner>,
    type_id: TypeId,
    /// 总线已关闭时为 `None`，此时 [`changed`](Self::changed) 立即返回 `Err(Closed)`。
    entry: Option<Arc<Entry>>,
}

impl Watcher {
    pub(crate) fn new(inner: Weak<Inner>, type_id: TypeId, entry: Arc<Entry>) -> Self {
        Self {
            inner,
            type_id,
            entry: Some(entry),
        }
    }

    /// 总线已关闭时得到的空句柄。
    pub(crate) fn inactive() -> Self {
        Self {
            inner: Weak::new(),
            type_id: TypeId::of::<()>(),
            entry: None,
        }
    }

    /// 等待“自上次返回以来至少发生过一次通知”。
    ///
    /// 通知先于调用到达时立即返回，多次通知被合并为一次；总线关闭后返回 `Err(Closed)`。
    pub async fn changed(&mut self) -> Result<(), Closed> {
        let Some(entry) = self.entry.as_ref() else {
            return Err(Closed);
        };
        let Kind::Watch { notify, closed } = &entry.kind else {
            return Err(Closed);
        };

        if closed.load(Ordering::Acquire) {
            return Err(Closed);
        }

        notify.notified().await;

        // 关闭优先于尚未消费的通知许可
        if closed.load(Ordering::Acquire) {
            return Err(Closed);
        }

        Ok(())
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        let Some(entry) = self.entry.take() else {
            return;
        };

        if let Some(inner) = self.inner.upgrade() {
            inner.remove(self.type_id, entry.id);
        }
    }
}

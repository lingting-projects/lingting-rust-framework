use crate::is_ntp;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex, OnceLock, Weak,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
};

struct Waiter {
    waker: Mutex<Option<Waker>>,
}

struct WaitNtp {
    waiter: Arc<Waiter>,
    registered: AtomicBool,
}

static WAITERS: OnceLock<Mutex<Vec<Weak<Waiter>>>> = OnceLock::new();

/// 等待首次 NTP 校时成功。
pub async fn wait_ntp() {
    WaitNtp::new().await;
}

impl WaitNtp {
    fn new() -> Self {
        Self {
            waiter: Arc::new(Waiter {
                waker: Mutex::new(None),
            }),
            registered: AtomicBool::new(false),
        }
    }
}

impl Future for WaitNtp {
    type Output = ();

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if is_ntp() {
            return Poll::Ready(());
        }

        if let Ok(mut waker) = self.waiter.waker.lock() {
            *waker = Some(context.waker().clone());
        }

        if !self.registered.swap(true, Ordering::AcqRel) {
            let waiters = WAITERS.get_or_init(|| Mutex::new(Vec::new()));
            if let Ok(mut waiters) = waiters.lock() {
                waiters.push(Arc::downgrade(&self.waiter));
            }
        }

        if is_ntp() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub(crate) fn wake_waiters() {
    let Some(waiters) = WAITERS.get() else {
        return;
    };

    let waiters = match waiters.lock() {
        Ok(mut waiters) => std::mem::take(&mut *waiters),
        Err(_) => return,
    };

    for waiter in waiters {
        let Some(waiter) = waiter.upgrade() else {
            continue;
        };
        let Ok(mut waker) = waiter.waker.lock() else {
            continue;
        };
        if let Some(waker) = waker.take() {
            waker.wake();
        }
    }
}

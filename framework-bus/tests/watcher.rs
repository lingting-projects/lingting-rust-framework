mod common;

use framework_bus::{Bus, Closed};
use futures::executor::block_on;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct A;

#[test]
fn notify_before_changed_returns_immediately() {
    let bus = Bus::new();
    let mut watcher = bus.watch::<A>();

    bus.notify::<A>();

    assert_eq!(block_on(watcher.changed()), Ok(()));
}

#[test]
fn repeated_notify_is_merged_for_watcher() {
    let bus = Bus::new();
    let mut watcher = bus.watch::<A>();

    bus.notify::<A>();
    bus.notify::<A>();
    bus.notify::<A>();

    assert_eq!(block_on(watcher.changed()), Ok(()));

    // 许可已被消费，再等不会立即返回
    assert!(block_on(poll_once(watcher.changed())).is_none());
}

#[test]
fn waiting_changed_is_woken_up() {
    let bus = Arc::new(Bus::new());
    let mut watcher = bus.watch::<A>();

    let notifier = Arc::clone(&bus);
    let handle = std::thread::spawn(move || {
        notifier.notify::<A>();
    });

    assert_eq!(block_on(watcher.changed()), Ok(()));
    handle.join().unwrap();
}

#[test]
fn dropping_watcher_unsubscribes() {
    let bus = Bus::new();

    let watcher = bus.watch::<A>();
    assert_eq!(bus.subscriber_count::<A>(), 1);

    drop(watcher);
    assert_eq!(bus.subscriber_count::<A>(), 0);
}

#[test]
fn close_wakes_waiting_changed() {
    let bus = Arc::new(Bus::new());
    let mut watcher = bus.watch::<A>();

    let closer = Arc::clone(&bus);
    let handle = std::thread::spawn(move || {
        closer.close();
    });

    assert_eq!(block_on(watcher.changed()), Err(Closed));
    handle.join().unwrap();
}

#[test]
fn watch_after_close_returns_closed() {
    let bus = Bus::new();
    bus.close();

    let mut watcher = bus.watch::<A>();

    assert_eq!(block_on(watcher.changed()), Err(Closed));
    assert_eq!(bus.subscriber_count::<A>(), 0);
}

#[test]
fn close_wins_over_pending_notification() {
    let bus = Bus::new();
    let mut watcher = bus.watch::<A>();

    bus.notify::<A>();
    bus.close();

    assert_eq!(block_on(watcher.changed()), Err(Closed));
}

#[test]
fn dropped_bus_closes_watcher() {
    let bus = Bus::new();
    let mut watcher = bus.watch::<A>();

    drop(bus);

    assert_eq!(block_on(watcher.changed()), Err(Closed));
}

#[test]
fn watcher_and_subscriber_share_count() {
    let bus = Bus::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_in_handler = Arc::clone(&calls);

    let _sub = bus.subscribe::<A>(move || {
        calls_in_handler.fetch_add(1, Ordering::SeqCst);
    });
    let _watcher = bus.watch::<A>();

    assert_eq!(bus.subscriber_count::<A>(), 2);

    bus.notify::<A>();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

/// 手动轮询一次 Future，用于确认“不会立即就绪”。
async fn poll_once<F: std::future::Future>(future: F) -> Option<F::Output> {
    use std::task::{Context, Poll};

    let mut future = Box::pin(future);
    let mut context = Context::from_waker(futures::task::noop_waker_ref());

    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => Some(output),
        Poll::Pending => None,
    }
}

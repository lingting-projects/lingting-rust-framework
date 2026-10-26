mod common;

use common::{logs, serialize};
use framework_bus::{Bus, Subscription};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

struct A;
struct B;

fn counter() -> (Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    (Arc::clone(&count), count)
}

#[test]
fn inline_dispatcher_runs_handler_once() {
    let bus = Bus::new();
    let (handle, count) = counter();

    let _sub = bus.subscribe::<A>(move || {
        handle.fetch_add(1, Ordering::SeqCst);
    });

    bus.notify::<A>();

    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn event_types_are_isolated() {
    let bus = Bus::new();
    let (handle_a, count_a) = counter();
    let (handle_b, count_b) = counter();

    let _sub_a = bus.subscribe::<A>(move || {
        handle_a.fetch_add(1, Ordering::SeqCst);
    });
    let _sub_b = bus.subscribe::<B>(move || {
        handle_b.fetch_add(1, Ordering::SeqCst);
    });

    bus.notify::<A>();

    assert_eq!(count_a.load(Ordering::SeqCst), 1);
    assert_eq!(count_b.load(Ordering::SeqCst), 0);
}

#[test]
fn all_subscribers_are_triggered() {
    let bus = Bus::new();
    let (handle_1, count_1) = counter();
    let (handle_2, count_2) = counter();

    let _sub_1 = bus.subscribe::<A>(move || {
        handle_1.fetch_add(1, Ordering::SeqCst);
    });
    let _sub_2 = bus.subscribe::<A>(move || {
        handle_2.fetch_add(1, Ordering::SeqCst);
    });

    assert_eq!(bus.subscriber_count::<A>(), 2);

    bus.notify::<A>();

    assert_eq!(count_1.load(Ordering::SeqCst), 1);
    assert_eq!(count_2.load(Ordering::SeqCst), 1);
}

#[test]
fn dropping_subscription_unsubscribes() {
    let bus = Bus::new();
    let (handle, count) = counter();

    let sub = bus.subscribe::<A>(move || {
        handle.fetch_add(1, Ordering::SeqCst);
    });
    drop(sub);

    assert_eq!(bus.subscriber_count::<A>(), 0);

    bus.notify::<A>();

    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[test]
fn notify_before_subscribe_is_not_replayed() {
    let bus = Bus::new();
    let (handle, count) = counter();

    bus.notify::<A>();

    let _sub = bus.subscribe::<A>(move || {
        handle.fetch_add(1, Ordering::SeqCst);
    });

    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[test]
fn handler_can_subscribe_notify_and_unsubscribe() {
    let bus = Arc::new(Bus::new());
    let own: Arc<Mutex<Option<Subscription>>> = Arc::new(Mutex::new(None));
    let called = Arc::new(AtomicUsize::new(0));
    let nested = Arc::new(AtomicUsize::new(0));

    let bus_in_handler = Arc::clone(&bus);
    let own_in_handler = Arc::clone(&own);
    let called_in_handler = Arc::clone(&called);
    let nested_in_handler = Arc::clone(&nested);

    let sub = bus.subscribe::<A>(move || {
        called_in_handler.fetch_add(1, Ordering::SeqCst);

        // 处理函数内再订阅、再通知、再退订自己
        let _extra = bus_in_handler.subscribe::<B>(|| {});
        bus_in_handler.notify::<B>();

        if let Some(sub) = own_in_handler.lock().unwrap().take() {
            drop(sub);
        }

        nested_in_handler.fetch_add(1, Ordering::SeqCst);
    });

    *own.lock().unwrap() = Some(sub);

    bus.notify::<A>();

    assert_eq!(called.load(Ordering::SeqCst), 1);
    assert_eq!(nested.load(Ordering::SeqCst), 1);
    assert_eq!(bus.subscriber_count::<A>(), 0);
}

#[test]
fn handler_panic_is_caught() {
    let _guard = serialize();

    let logs = logs();
    logs.clear();

    let bus = Bus::new();
    let (handle, count) = counter();

    let _panicking = bus.subscribe::<A>(|| panic!("测试用 panic"));
    let _other = bus.subscribe::<A>(move || {
        handle.fetch_add(1, Ordering::SeqCst);
    });

    bus.notify::<A>();

    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(logs.count_containing(log::Level::Error, "panicked"), 1);

    bus.notify::<A>();

    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[test]
fn detached_subscription_survives_handle_drop() {
    let bus = Bus::new();
    let (handle, count) = counter();

    bus.subscribe::<A>(move || {
        handle.fetch_add(1, Ordering::SeqCst);
    })
    .detach();

    assert_eq!(bus.subscriber_count::<A>(), 1);

    bus.notify::<A>();

    assert_eq!(count.load(Ordering::SeqCst), 1);

    bus.close();

    assert_eq!(bus.subscriber_count::<A>(), 0);

    bus.notify::<A>();

    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn handles_are_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<Bus>();
    assert_send_sync::<Subscription>();
    assert_send_sync::<framework_bus::Watcher>();
}

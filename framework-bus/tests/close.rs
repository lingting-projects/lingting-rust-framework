mod common;

use common::{logs, serialize};
use framework_bus::Bus;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct A;
struct B;

#[test]
fn close_is_idempotent() {
    let bus = Bus::new();

    assert!(!bus.is_closed());

    bus.close();
    bus.close();
    bus.close();

    assert!(bus.is_closed());
}

#[test]
fn notify_after_close_is_ignored_with_warning() {
    let _guard = serialize();

    let logs = logs();
    logs.clear();

    let bus = Bus::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_in_handler = Arc::clone(&calls);

    let _sub = bus.subscribe::<A>(move || {
        calls_in_handler.fetch_add(1, Ordering::SeqCst);
    });

    bus.close();

    bus.notify::<A>();
    bus.notify::<A>();

    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(logs.count_containing(log::Level::Warn, "closed bus"), 2);
}

#[test]
fn notify_without_subscribers_is_silent() {
    let _guard = serialize();

    let logs = logs();
    logs.clear();

    let bus = Bus::new();
    bus.notify::<A>();
    bus.notify::<B>();

    assert_eq!(logs.count_containing(log::Level::Warn, "closed bus"), 0);
    assert_eq!(logs.count_containing(log::Level::Warn, "framework-bus"), 0);
}

#[test]
fn subscribe_after_close_is_ignored_with_warning() {
    let _guard = serialize();

    let logs = logs();
    logs.clear();

    let bus = Bus::new();
    bus.close();

    let _sub = bus.subscribe::<A>(|| {});

    assert_eq!(bus.subscriber_count::<A>(), 0);
    assert_eq!(logs.count_containing(log::Level::Warn, "subscribe"), 1);
}

#[test]
fn close_drops_all_subscriptions() {
    let bus = Bus::new();

    let _sub_a = bus.subscribe::<A>(|| {});
    let _sub_b = bus.subscribe::<B>(|| {});
    let _watcher = bus.watch::<A>();

    assert_eq!(bus.subscriber_count::<A>(), 2);
    assert_eq!(bus.subscriber_count::<B>(), 1);

    bus.close();

    assert_eq!(bus.subscriber_count::<A>(), 0);
    assert_eq!(bus.subscriber_count::<B>(), 0);
}

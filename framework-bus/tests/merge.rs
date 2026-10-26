mod common;

use common::QueueDispatcher;
use framework_bus::Bus;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct A;

fn counter() -> (Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    (Arc::clone(&count), count)
}

#[test]
fn repeated_notify_before_run_is_merged() {
    let dispatcher = Arc::new(QueueDispatcher::default());
    let bus = Bus::with_dispatcher(Arc::clone(&dispatcher) as Arc<_>);
    let (handle, count) = counter();

    let _sub = bus.subscribe::<A>(move || {
        handle.fetch_add(1, Ordering::SeqCst);
    });

    for _ in 0..10 {
        bus.notify::<A>();
    }

    assert_eq!(dispatcher.len(), 1);

    dispatcher.run_all();

    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(dispatcher.len(), 0);
}

#[test]
fn notify_after_run_produces_new_job() {
    let dispatcher = Arc::new(QueueDispatcher::default());
    let bus = Bus::with_dispatcher(Arc::clone(&dispatcher) as Arc<_>);
    let (handle, count) = counter();

    let _sub = bus.subscribe::<A>(move || {
        handle.fetch_add(1, Ordering::SeqCst);
    });

    bus.notify::<A>();
    dispatcher.run_all();

    assert_eq!(count.load(Ordering::SeqCst), 1);

    bus.notify::<A>();
    dispatcher.run_all();

    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[test]
fn notify_during_handler_triggers_again() {
    let dispatcher = Arc::new(QueueDispatcher::default());
    let bus = Bus::with_dispatcher(Arc::clone(&dispatcher) as Arc<_>);
    let calls = Arc::new(AtomicUsize::new(0));

    let bus_in_handler = Arc::new(bus.clone());
    let calls_in_handler = Arc::clone(&calls);
    let dispatcher_in_handler = Arc::clone(&dispatcher);

    let _sub = bus.subscribe::<A>(move || {
        calls_in_handler.fetch_add(1, Ordering::SeqCst);

        // 处理函数执行期间再发一次通知，应产生新的任务
        bus_in_handler.notify::<A>();

        assert_eq!(dispatcher_in_handler.len(), 1);
    });

    bus.notify::<A>();
    assert_eq!(dispatcher.len(), 1);

    // 第一次执行期间收到通知，产生第二个任务
    dispatcher.run_all();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(dispatcher.len(), 1);

    dispatcher.run_all();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn dropped_job_resets_pending() {
    let dispatcher = Arc::new(QueueDispatcher::default());
    let bus = Bus::with_dispatcher(Arc::clone(&dispatcher) as Arc<_>);
    let (handle, count) = counter();

    let _sub = bus.subscribe::<A>(move || {
        handle.fetch_add(1, Ordering::SeqCst);
    });

    bus.notify::<A>();
    assert_eq!(dispatcher.len(), 1);

    // 任务未执行就被丢弃
    dispatcher.drop_all();

    bus.notify::<A>();
    assert_eq!(dispatcher.len(), 1);

    dispatcher.run_all();
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

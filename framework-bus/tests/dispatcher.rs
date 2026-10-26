mod common;

use common::{CountingDispatcher, QueueDispatcher};
use framework_bus::{Bus, Dispatcher, InlineDispatcher, Job};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct A;

fn counter() -> (Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    (Arc::clone(&count), count)
}

#[test]
fn set_dispatcher_switches_new_notifications() {
    let old = Arc::new(CountingDispatcher::default());
    let bus = Bus::with_dispatcher(Arc::clone(&old) as Arc<dyn Dispatcher>);
    let (handle, count) = counter();

    let _sub = bus.subscribe::<A>(move || {
        handle.fetch_add(1, Ordering::SeqCst);
    });

    bus.notify::<A>();
    assert_eq!(old.count(), 1);

    let new = Arc::new(QueueDispatcher::default());
    bus.set_dispatcher(Arc::clone(&new) as Arc<dyn Dispatcher>);

    bus.notify::<A>();
    assert_eq!(old.count(), 1);
    assert_eq!(new.len(), 1);
    assert_eq!(count.load(Ordering::SeqCst), 1);

    new.run_all();
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[test]
fn concurrent_notify_from_threads() {
    let bus = Arc::new(Bus::with_dispatcher(Arc::new(InlineDispatcher)));
    let (handle, count) = counter();

    let _sub = bus.subscribe::<A>(move || {
        handle.fetch_add(1, Ordering::SeqCst);
    });

    let threads: Vec<_> = (0..8)
        .map(|_| {
            let bus = Arc::clone(&bus);
            std::thread::spawn(move || {
                for _ in 0..100 {
                    bus.notify::<A>();
                }
            })
        })
        .collect();

    for thread in threads {
        thread.join().unwrap();
    }

    assert!(count.load(Ordering::SeqCst) >= 1);
}

#[test]
fn job_dropped_without_run_keeps_subscription_alive() {
    let queue = Arc::new(QueueDispatcher::default());
    let bus = Bus::with_dispatcher(Arc::clone(&queue) as Arc<dyn Dispatcher>);

    bus.notify::<A>();
    queue.drop_all();

    assert_eq!(bus.subscriber_count::<A>(), 0);
}

#[test]
fn dispatcher_can_move_job_to_another_thread() {
    let bus = Bus::new();
    let (handle, count) = counter();

    let _sub = bus.subscribe::<A>(move || {
        handle.fetch_add(1, Ordering::SeqCst);
    });

    // Job 自身满足 Send，可交给其他线程执行
    let (sender, receiver) = std::sync::mpsc::channel::<Job>();

    struct ChannelDispatcher(std::sync::Mutex<std::sync::mpsc::Sender<Job>>);

    impl Dispatcher for ChannelDispatcher {
        fn dispatch(&self, job: Job) {
            self.0.lock().unwrap().send(job).unwrap();
        }
    }

    bus.set_dispatcher(Arc::new(ChannelDispatcher(std::sync::Mutex::new(sender))));

    bus.notify::<A>();

    let job = receiver.recv().unwrap();
    std::thread::spawn(move || job.run()).join().unwrap();

    assert_eq!(count.load(Ordering::SeqCst), 1);
}

use crate::{TimerNameLike, TimerRegistration, TimerResult, TimerTarget, TimerWrapper, timer_iter};
use anyhow::{Result, anyhow};
use framework_core::current_millis;
use futures_util::FutureExt;
use std::any::Any;
use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, OnceLock};
use tokio::sync::Semaphore;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::task::{Id, JoinSet};
use tokio::time::{Duration, sleep};

static WAKE_SENDER: OnceLock<UnboundedSender<String>> = OnceLock::new();

/// 定时任务运行时的基础配置。
#[derive(Clone, Copy, Debug, Default)]
pub struct TimerOption {
    /// 同时运行的任务数上限；`None` 表示不设上限。
    pub max_concurrent_tasks: Option<usize>,
    /// 未在宏上单独指定时，任务 panic 后是否立即重试。
    pub retry_immediately_on_panic: bool,
}

struct TimerState {
    name: String,
    registration: &'static TimerRegistration,
    next_run_at: i64,
    running: bool,
}

/// 启动所有已注册的定时任务。
///
/// 调度器等待至最近任务的下次执行时间，并将到期任务交给独立 Tokio 子任务执行，
/// 因此某个任务的执行不会阻塞其他任务的调度。
pub fn start(wrapper: Option<TimerWrapper>, option: TimerOption) {
    let semaphore = option.max_concurrent_tasks.map(|count| {
        assert!(count > 0, "max_concurrent_tasks 必须大于 0");
        Arc::new(Semaphore::new(count))
    });
    let states = timer_iter()
        .flat_map(|registration| {
            (1..=registration.count).map(move |index| TimerState {
                name: if registration.count == 1 {
                    registration.name.to_owned()
                } else {
                    format!("{}_{index}", registration.name)
                },
                registration,
                next_run_at: 0,
                running: false,
            })
        })
        .collect::<Vec<_>>();
    let (wake_sender, wake_receiver) = unbounded_channel();
    WAKE_SENDER
        .set(wake_sender)
        .expect("定时任务运行时只能启动一次");

    tokio::spawn(run_scheduler(
        states,
        wake_receiver,
        wrapper,
        option,
        semaphore,
    ));
}

/// 立即唤醒名称匹配的任一空闲定时任务实例。
pub fn wake(name: impl TimerNameLike) {
    let Some(sender) = WAKE_SENDER.get() else {
        return;
    };
    let _ = sender.send(name.timer_name().to_owned());
}

async fn run_scheduler(
    mut states: Vec<TimerState>,
    mut wake_receiver: UnboundedReceiver<String>,
    wrapper: Option<TimerWrapper>,
    option: TimerOption,
    semaphore: Option<Arc<Semaphore>>,
) {
    let mut tasks = JoinSet::new();
    let mut task_indices = HashMap::new();

    loop {
        let wait_millis = dispatch_due_tasks(
            &mut states,
            &mut tasks,
            &mut task_indices,
            &wrapper,
            semaphore.as_ref(),
            current_millis().expect("获取当前毫秒时间戳失败"),
        );

        match wait_millis {
            Some(wait_millis) => {
                tokio::select! {
                    Some(name) = wake_receiver.recv() => wake_timer(&mut states, name),
                    result = tasks.join_next_with_id(), if !tasks.is_empty() => {
                        if let Some(result) = result {
                            handle_task_result(result, &mut task_indices, &mut states, option);
                        }
                    }
                    _ = sleep(Duration::from_millis(wait_millis)) => {}
                    else => return,
                }
            }
            None => {
                tokio::select! {
                    Some(name) = wake_receiver.recv() => wake_timer(&mut states, name),
                    result = tasks.join_next_with_id() => {
                        if let Some(result) = result {
                            handle_task_result(result, &mut task_indices, &mut states, option);
                        }
                    }
                    else => return,
                }
            }
        }
    }
}

fn wake_timer(states: &mut [TimerState], name: String) {
    if let Some(state) = states
        .iter_mut()
        .find(|state| !state.running && state.registration.name == name)
    {
        state.next_run_at = -10086;
    }
}

fn dispatch_due_tasks(
    states: &mut [TimerState],
    tasks: &mut JoinSet<Result<TimerResult>>,
    task_indices: &mut HashMap<Id, usize>,
    wrapper: &Option<TimerWrapper>,
    semaphore: Option<&Arc<Semaphore>>,
    now: i64,
) -> Option<u64> {
    let mut next_run_at: Option<i64> = None;
    for (index, state) in states.iter_mut().enumerate() {
        if state.running {
            continue;
        }
        if state.next_run_at > now {
            next_run_at =
                Some(next_run_at.map_or(state.next_run_at, |next| next.min(state.next_run_at)));
            continue;
        }

        state.running = true;
        let target = state.registration.target;
        let wrapper = wrapper.clone();
        let semaphore = semaphore.cloned();
        let task = tasks.spawn(async move {
            execute_timer(target, wrapper.as_ref(), semaphore.as_ref()).await
        });
        task_indices.insert(task.id(), index);
    }
    next_run_at.map(|next_run_at| next_run_at.saturating_sub(now).max(0) as u64)
}

fn handle_task_result(
    result: Result<(Id, Result<TimerResult>), tokio::task::JoinError>,
    task_indices: &mut HashMap<Id, usize>,
    states: &mut [TimerState],
    option: TimerOption,
) {
    let (task_id, result) = match result {
        Ok((task_id, result)) => (task_id, result),
        Err(error) => {
            log::error!("定时任务 Tokio 子任务执行失败：{error:?}");
            (
                error.id(),
                Err(anyhow!("定时任务 Tokio 子任务执行失败：{error:?}")),
            )
        }
    };
    let index = task_indices
        .remove(&task_id)
        .expect("定时任务缺少任务实例索引");
    let state = &mut states[index];
    state.running = false;

    let now = current_millis().expect("获取当前毫秒时间戳失败");
    let next = match result {
        Ok(result) => {
            if let Some(error) = result.error {
                log::error!("定时任务 {} 执行失败：{error:?}", state.name);
            }
            match result.next {
                None => now.saturating_add(state.registration.interval),
                Some(v) => v,
            }
        }
        Err(error) => {
            log::error!("定时任务 {} 发生 panic: {error:?}", state.name);
            if state
                .registration
                .retry_immediately_on_panic
                .unwrap_or(option.retry_immediately_on_panic)
            {
                0
            } else {
                now.saturating_add(state.registration.interval)
            }
        }
    };
    state.next_run_at = next
}

async fn execute_timer(
    target: TimerTarget,
    wrapper: Option<&TimerWrapper>,
    semaphore: Option<&Arc<Semaphore>>,
) -> Result<TimerResult> {
    let _permit = match semaphore {
        Some(semaphore) => Some(
            semaphore
                .acquire()
                .await
                .map_err(|_| anyhow!("定时任务并发控制 Semaphore 已关闭"))?,
        ),
        None => None,
    };

    let future = match wrapper {
        Some(wrapper) => wrapper(target),
        None => target(),
    };

    AssertUnwindSafe(future)
        .catch_unwind()
        .await
        .map_err(panic_to_error)
}

fn panic_to_error(payload: Box<dyn Any + Send>) -> anyhow::Error {
    let message = if let Some(message) = payload.downcast_ref::<&str>() {
        *message
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.as_str()
    } else {
        "未知 panic payload"
    };

    anyhow!("定时任务执行发生 panic: {message}")
}

use anyhow::{Context, Result, anyhow, ensure};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use wasm_bindgen::{JsCast, closure::Closure, prelude::*};

/// 浏览器校时配置。
#[derive(Clone, Debug)]
pub struct NtpConfig {
    /// 浏览器定时器调用时间提供函数的间隔。
    pub sync_interval: Duration,
}

impl Default for NtpConfig {
    fn default() -> Self {
        Self {
            sync_interval: Duration::from_secs(10 * 60),
        }
    }
}

struct NtpState {
    _timer_id: i32,
    _callback: Closure<dyn FnMut()>,
}

thread_local! {
    static NTP_STATE: RefCell<Option<NtpState>> = const { RefCell::new(None) };
}

/// 保证浏览器校时定时器只启动一次。
static INITIALIZED: AtomicBool = AtomicBool::new(false);

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_name = setInterval)]
    fn set_interval(callback: &js_sys::Function, timeout: i32)
                    -> std::result::Result<i32, JsValue>;
}

/// 启动浏览器定时校时任务。
///
/// `provider` 返回已校准的 Unix 毫秒绝对时间戳。初始化会立即调用一次，回调失败不会停止后续重试。
pub fn init_ntp<F>(config: NtpConfig, provider: F) -> Result<()>
where
    F: Fn() -> Result<u128> + 'static,
{
    let timeout = validate_config(&config)?;

    if INITIALIZED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Ok(());
    }

    let provider = Rc::new(provider);
    let timer_provider = Rc::clone(&provider);
    let callback = Closure::new(move || {
        let _ = synchronize(&*timer_provider);
    });
    let timer_id = match set_interval(callback.as_ref().unchecked_ref(), timeout) {
        Ok(timer_id) => timer_id,
        Err(error) => {
            INITIALIZED.store(false, Ordering::Release);
            return Err(anyhow!("创建浏览器校时定时器失败：{error:?}"));
        }
    };

    NTP_STATE.with(|state| {
        state.replace(Some(NtpState {
            _timer_id: timer_id,
            _callback: callback,
        }));
    });
    let _ = synchronize(&*provider);

    Ok(())
}

fn validate_config(config: &NtpConfig) -> Result<i32> {
    let millis = config.sync_interval.as_millis();
    ensure!(millis > 0, "NTP 同步间隔至少为 1 毫秒");
    ensure!(
        millis <= i32::MAX as u128,
        "NTP 同步间隔不能超过 {} 毫秒",
        i32::MAX
    );

    i32::try_from(millis).context("NTP 同步间隔超过浏览器定时器范围")
}

fn synchronize(provider: &impl Fn() -> Result<u128>) -> Result<()> {
    let unix_millis = provider()?;
    let unix_millis = i128::try_from(unix_millis).context("校准时间戳超过 i128 范围")?;

    super::apply_ntp_time(unix_millis)
}

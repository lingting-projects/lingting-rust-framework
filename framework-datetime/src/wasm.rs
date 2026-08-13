#[cfg(feature = "ntp")]
mod ntp;

use anyhow::{Context, Result};
use arc_swap::ArcSwap;
#[cfg(feature = "ntp")]
use std::sync::Arc;
use std::sync::{
    OnceLock,
    atomic::{AtomicBool, Ordering},
};
use web_time::Instant;

/// 内部时钟锚点。Unix 时间戳使用 i128，避免 Epoch 前时间无法表示。
struct ClockAnchor {
    unix_millis: i128,
    instant: Instant,
}

static CLOCK_ANCHOR: OnceLock<ArcSwap<ClockAnchor>> = OnceLock::new();

/// 是否至少成功完成过一次由调用方提供的校时。
static NTP_SYNCED: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "ntp")]
pub use crate::waiter::wait_ntp;
#[cfg(feature = "ntp")]
pub use ntp::{NtpConfig, init_ntp};

/// 获取经过校时修正后的 Unix 毫秒时间戳。
pub fn current_millis() -> Result<i64> {
    i64::try_from(current_millis_i128()?).context("修正后的时间超出 i64 范围")
}

/// 获取经过校时修正后的有符号 Unix 毫秒时间戳。
pub fn current_millis_i128() -> Result<i128> {
    let anchor = clock_anchor()?.load();
    let elapsed_millis = i128::try_from(anchor.instant.elapsed().as_millis())
        .context("程序运行时间超过 i128 范围")?;

    anchor
        .unix_millis
        .checked_add(elapsed_millis)
        .context("内部时间戳溢出")
}

/// 当前时间是否已经至少成功完成过一次调用方提供的校时。
pub fn is_ntp() -> bool {
    NTP_SYNCED.load(Ordering::Acquire)
}

#[cfg(feature = "ntp")]
pub(crate) fn apply_ntp_time(unix_millis: i128) -> Result<()> {
    let anchor = clock_anchor()?;
    anchor.store(Arc::new(ClockAnchor {
        unix_millis,
        instant: Instant::now(),
    }));
    NTP_SYNCED.store(true, Ordering::Release);
    crate::wake_waiters();

    Ok(())
}

fn clock_anchor() -> Result<&'static ArcSwap<ClockAnchor>> {
    if let Some(anchor) = CLOCK_ANCHOR.get() {
        return Ok(anchor);
    }

    let new_anchor = ClockAnchor {
        unix_millis: system_unix_millis()?,
        instant: Instant::now(),
    };
    let _ = CLOCK_ANCHOR.set(ArcSwap::from_pointee(new_anchor));

    CLOCK_ANCHOR.get().context("初始化内部时钟锚点失败")
}

fn system_unix_millis() -> Result<i128> {
    let millis = js_sys::Date::now();
    if !millis.is_finite() {
        anyhow::bail!("浏览器时间戳不是有限数值");
    }

    Ok(millis as i128)
}

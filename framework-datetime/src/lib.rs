#[cfg(feature = "ntp")]
mod ntp;

use anyhow::{Context, Result};
use arc_swap::ArcSwap;
#[cfg(feature = "ntp")]
use std::sync::Arc;
use std::{
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

/// 内部时钟锚点。Unix 时间戳使用 i128，避免 Epoch 前时间无法表示。
struct ClockAnchor {
    unix_millis: i128,
    instant: Instant,
}

static CLOCK_ANCHOR: OnceLock<ArcSwap<ClockAnchor>> = OnceLock::new();

/// 是否至少成功完成过一次 NTP 校时。
static NTP_SYNCED: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "ntp")]
pub use ntp::{NTP_SERVERS, NtpConfig, init_ntp, wait_ntp};

/// 获取经过 NTP 差值修正后的 Unix 毫秒时间戳。
pub fn current_millis() -> Result<u64> {
    let millis = current_millis_i128()?;

    u64::try_from(millis).context("修正后的时间早于 Unix Epoch 或超过 u64 范围")
}

/// 获取经过 NTP 差值修正后的 Unix 毫秒时间戳。
pub fn current_millis_u128() -> Result<u128> {
    let millis = current_millis_i128()?;

    u128::try_from(millis).context("修正后的时间早于 Unix Epoch")
}

/// 获取经过 NTP 差值修正后的有符号 Unix 毫秒时间戳。
pub fn current_millis_i128() -> Result<i128> {
    let anchor = clock_anchor()?.load();
    let elapsed_millis = i128::try_from(anchor.instant.elapsed().as_millis())
        .context("程序运行时间超过 i128 范围")?;

    anchor
        .unix_millis
        .checked_add(elapsed_millis)
        .context("内部时间戳溢出")
}

/// 当前时间是否已经基于至少一次成功的 NTP 校时进行修正。
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
    ntp::wake_waiters();

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

pub(crate) fn system_unix_millis() -> Result<i128> {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => i128::try_from(duration.as_millis()).context("系统时间戳超过 i128 范围"),
        Err(error) => {
            let millis =
                i128::try_from(error.duration().as_millis()).context("系统时间戳超过 i128 范围")?;

            Ok(-millis)
        }
    }
}

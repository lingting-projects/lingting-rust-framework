mod waiter;

#[cfg(not(feature = "tokio"))]
mod sync_std;
#[cfg(feature = "tokio")]
mod sync_tokio;

use anyhow::{Context, Result, anyhow, ensure};
use rsntp::SntpDuration;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

pub use waiter::wait_ntp;
pub(crate) use waiter::wake_waiters;

/// 内置 NTP 服务列表。
pub const NTP_SERVERS: &[&str] = &[
    "time.cloudflare.com",
    "time.google.com",
    "pool.ntp.org",
    "time.windows.com",
    "time.nist.gov",
    "time.apple.com",
    "time.asia.apple.com",
    "cn.ntp.org.cn",
    "ntp.ntsc.ac.cn",
    "cn.pool.ntp.org",
    "ntp.aliyun.com",
    "ntp1.aliyun.com",
    "ntp2.aliyun.com",
    "ntp3.aliyun.com",
    "ntp4.aliyun.com",
    "ntp5.aliyun.com",
    "ntp6.aliyun.com",
    "ntp7.aliyun.com",
];

/// NTP 校时配置。
#[derive(Clone, Debug)]
pub struct NtpConfig {
    pub servers: Vec<String>,
    pub request_timeout: Duration,
    pub sync_interval: Duration,
    pub retry_interval: Duration,
}

impl Default for NtpConfig {
    fn default() -> Self {
        Self {
            servers: NTP_SERVERS
                .iter()
                .map(|server| (*server).to_owned())
                .collect(),
            request_timeout: Duration::from_secs(5),
            sync_interval: Duration::from_secs(10 * 60),
            retry_interval: Duration::from_secs(5),
        }
    }
}

/// 保证 NTP 后台任务只启动一次。
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// 启动常驻 NTP 校时任务。
pub fn init_ntp(config: NtpConfig) -> Result<()> {
    validate_config(&config)?;

    if INITIALIZED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Ok(());
    }

    let result = start_sync_task(config);
    if result.is_err() {
        INITIALIZED.store(false, Ordering::Release);
    }

    result
}

#[cfg(not(feature = "tokio"))]
fn start_sync_task(config: NtpConfig) -> Result<()> {
    thread::Builder::new()
        .name("framework-datetime-ntp".to_owned())
        .spawn(move || sync_std::sync_loop(config))
        .map(|_| ())
        .map_err(Into::into)
}

#[cfg(feature = "tokio")]
fn start_sync_task(config: NtpConfig) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    thread::Builder::new()
        .name("framework-datetime-ntp".to_owned())
        .spawn(move || runtime.block_on(sync_tokio::sync_loop(config)))
        .map(|_| ())
        .map_err(Into::into)
}

fn validate_config(config: &NtpConfig) -> Result<()> {
    ensure!(!config.servers.is_empty(), "NTP 服务列表不能为空");
    ensure!(
        !config.request_timeout.is_zero(),
        "NTP 请求超时时间必须大于零"
    );
    ensure!(!config.sync_interval.is_zero(), "NTP 同步间隔必须大于零");
    ensure!(!config.retry_interval.is_zero(), "NTP 重试间隔必须大于零");

    Ok(())
}

pub(super) fn ntp_unix_millis(duration: SntpDuration) -> Result<i128> {
    let millis = duration
        .abs_as_std_duration()
        .map_err(|error| anyhow!(error))?
        .as_millis();
    let millis = i128::try_from(millis).map_err(|error| anyhow!(error))?;
    let offset_millis = if duration.signum() < 0 {
        -millis
    } else {
        millis
    };

    crate::system_unix_millis()?
        .checked_add(offset_millis)
        .context("校准后的 NTP 时间戳溢出")
}

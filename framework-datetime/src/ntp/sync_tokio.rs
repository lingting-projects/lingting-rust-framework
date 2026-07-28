use super::{NtpConfig, ntp_unix_millis};
use anyhow::{Context, Result, anyhow};
use rsntp::AsyncSntpClient;
use tokio::{
    task::JoinSet,
    time::{sleep, timeout},
};

pub(super) async fn sync_loop(config: NtpConfig) {
    loop {
        let delay = match query_ntp_offset_millis(&config).await {
            Ok(unix_millis) => match crate::native::apply_ntp_time(unix_millis) {
                Ok(()) => config.sync_interval,
                Err(_) => config.retry_interval,
            },
            Err(_) => config.retry_interval,
        };

        sleep(delay).await;
    }
}

async fn query_ntp_offset_millis(config: &NtpConfig) -> Result<i128> {
    let mut tasks = JoinSet::new();

    for server in &config.servers {
        let server = server.clone();
        let request_timeout = config.request_timeout;

        tasks.spawn(async move { query_server(server, request_timeout).await });
    }

    let mut errors = Vec::with_capacity(config.servers.len());
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(offset)) => {
                tasks.abort_all();
                return Ok(offset);
            }
            Ok(Err(error)) => errors.push(format!("{error:#}")),
            Err(error) => errors.push(format!("NTP 查询任务异常退出：{error}")),
        }
    }

    Err(anyhow!("所有 NTP 服务器均同步失败：{}", errors.join("; ")))
}

async fn query_server(server: String, request_timeout: std::time::Duration) -> Result<i128> {
    let client = AsyncSntpClient::new();
    let result = timeout(request_timeout, client.synchronize(server.as_str()))
        .await
        .map_err(|_| anyhow!("{server}: NTP 请求超时"))?
        .with_context(|| format!("{server}: NTP 同步失败"))?;

    ntp_unix_millis(result.clock_offset()).with_context(|| format!("{server}: 计算 NTP 时间戳失败"))
}

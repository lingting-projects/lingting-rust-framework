use super::{NtpConfig, ntp_unix_millis};
use anyhow::{Context, Result, anyhow};
use rsntp::SntpClient;
use std::{sync::mpsc, thread, time::Duration};

pub(super) fn sync_loop(config: NtpConfig) {
    loop {
        let delay = match query_ntp_offset_millis(&config) {
            Ok(unix_millis) => match crate::apply_ntp_time(unix_millis) {
                Ok(()) => config.sync_interval,
                Err(_) => config.retry_interval,
            },
            Err(_) => config.retry_interval,
        };

        thread::sleep(delay);
    }
}

fn query_ntp_offset_millis(config: &NtpConfig) -> Result<i128> {
    let (sender, receiver) = mpsc::channel();

    for server in &config.servers {
        let sender = sender.clone();
        let server = server.clone();
        let server_for_error = server.clone();
        let timeout = config.request_timeout;

        thread::Builder::new()
            .name("framework-datetime-ntp-query".to_owned())
            .spawn(move || {
                let _ = sender.send(query_server(&server, timeout));
            })
            .with_context(|| format!("创建 NTP 查询线程失败：{server_for_error}"))?;
    }
    drop(sender);

    let mut errors = Vec::with_capacity(config.servers.len());
    for result in receiver {
        match result {
            Ok(offset) => return Ok(offset),
            Err(error) => errors.push(format!("{error:#}")),
        }
    }

    Err(anyhow!("所有 NTP 服务器均同步失败：{}", errors.join("; ")))
}

fn query_server(server: &str, timeout: Duration) -> Result<i128> {
    let mut client = SntpClient::new();
    client.set_timeout(timeout);

    let result = client
        .synchronize(server)
        .with_context(|| format!("{server}: NTP 同步失败"))?;

    ntp_unix_millis(result.clock_offset()).with_context(|| format!("{server}: 计算 NTP 时间戳失败"))
}

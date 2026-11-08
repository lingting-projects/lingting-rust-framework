//! 请求重试。
//!
//! 服务端在并发压力下会短暂回 429 / 503，网络本身也会偶发断流。这类失败几乎都是瞬时的：
//! 等一会儿重试同一个请求就能成功，重试整个文件则会把已经下好的部分全部丢掉。
//! 因此重试只包在单个请求外面，由调用方决定重试哪一段。
//!
//! 退避优先听服务器的：响应带 `retry-after` 就按它等，没有才用指数退避。
//! 重试次数有上限，否则一个真的挂掉的地址会把整个下载卡死。

use std::time::Duration;

use anyhow::Result;
use tokio::time::sleep;

use crate::backend::{self, Client};
use crate::method::HttpMethod;
use crate::request::HttpRequest;
use crate::response::HttpResponse;
use crate::stream::HttpStreamResponse;

/// 最多尝试的次数，含第一次。
pub(crate) const MAX_ATTEMPTS: u32 = 4;

/// 指数退避的基准间隔，第 `n` 次重试等 `BASE_DELAY * 2^(n-1)`。
const BASE_DELAY: Duration = Duration::from_millis(400);

/// 单次等待的上限：服务器偶尔会回一个很大的 `retry-after`，等太久不如直接失败。
const MAX_DELAY: Duration = Duration::from_secs(8);

/// 这个状态码是否值得重试。
///
/// 429 是限流，502 / 503 / 504 是网关与服务的瞬时故障；其余 4xx 是请求本身有问题，
/// 重试多少次都一样，只会浪费时间。
pub(crate) fn retryable_status(status: u16) -> bool {
    matches!(status, 429 | 502 | 503 | 504)
}

/// 带重试地发起一次 HEAD 请求。
pub(crate) async fn head(client: &Client, request: HttpRequest) -> Result<HttpResponse> {
    let mut attempt = 0;

    loop {
        attempt += 1;
        match backend::send(client, HttpMethod::Head, request.clone()).await {
            Ok(response) if retryable_status(response.status) => {
                if attempt >= MAX_ATTEMPTS {
                    return Ok(response);
                }
                let delay = delay_of(attempt, response.header("retry-after").map(String::as_str));
                sleep(delay).await;
            }
            Ok(response) => return Ok(response),
            Err(error) => {
                if attempt >= MAX_ATTEMPTS {
                    return Err(error);
                }
                sleep(delay_of(attempt, None)).await;
            }
        }
    }
}

/// 带重试地发起一次流式 GET 请求。
///
/// 只重试「请求失败」与「状态码可重试」两种情况。拿到响应之后由调用方读流，
/// 读流中途断开不在这里处理：那种情况重试的是整个分块，由分块下载自己决定。
pub(crate) async fn stream(client: &Client, request: HttpRequest) -> Result<HttpStreamResponse> {
    let mut attempt = 0;

    loop {
        attempt += 1;
        match backend::stream(client, HttpMethod::Get, request.clone()).await {
            Ok(stream) if retryable_status(stream.status) => {
                if attempt >= MAX_ATTEMPTS {
                    return Ok(stream);
                }
                let delay = delay_of(attempt, stream.header("retry-after").map(String::as_str));
                tracing::warn!(
                    "{} 返回 {}，{} 毫秒后重试（第 {attempt} 次）",
                    request.url,
                    stream.status,
                    delay.as_millis()
                );
                sleep(delay).await;
            }
            Ok(stream) => return Ok(stream),
            Err(error) => {
                if attempt >= MAX_ATTEMPTS {
                    return Err(error);
                }
                let delay = delay_of(attempt, None);
                tracing::warn!(
                    "请求 {} 失败，{} 毫秒后重试（第 {attempt} 次）: {error:#}",
                    request.url,
                    delay.as_millis()
                );
                sleep(delay).await;
            }
        }
    }
}

/// 第 `attempt` 次重试的指数退避间隔。
///
/// 分块重试用它：那一层拿不到响应头，只能按次数退避。
pub(crate) fn backoff(attempt: u32) -> Duration {
    let factor = 1_u32 << (attempt.saturating_sub(1)).min(4);
    (BASE_DELAY * factor).min(MAX_DELAY)
}

/// 第 `attempt` 次重试要等多久。
///
/// 服务器给了 `retry-after` 就按它等（只支持秒数形式，HTTP 日期形式罕见且不值得为此
/// 引入日期解析），但不超过 [`MAX_DELAY`]；没给就按指数退避。
fn delay_of(attempt: u32, retry_after: Option<&str>) -> Duration {
    if let Some(seconds) = retry_after.and_then(parse_seconds) {
        return seconds.min(MAX_DELAY);
    }

    backoff(attempt)
}

/// `retry-after` 头里的秒数。
fn parse_seconds(value: &str) -> Option<Duration> {
    let seconds: u64 = value.trim().parse().ok()?;
    Some(Duration::from_secs(seconds))
}

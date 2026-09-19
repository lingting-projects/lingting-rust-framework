# framework-datetime

提供 Unix 毫秒时间戳，并可选地进行持续校时。

内部时钟使用 Unix 毫秒时间戳和 `Instant` 组成锚点。首次读取建立锚点，后续读取由单调时钟推进，因此常规读取不会受到系统时钟回拨或前跳影响。校时成功后会替换锚点，以校准时间为准。

## 安装

默认仅提供本地时间读取：

```toml
[dependencies]
framework-datetime = { path = "../framework-datetime" }
```

启用校时能力：

```toml
[dependencies]
framework-datetime = { path = "../framework-datetime", features = ["ntp"] }
```

非 JS/wasm 平台可选启用 Tokio 管理 NTP 并发查询：

```toml
[dependencies]
framework-datetime = { path = "../framework-datetime", features = ["tokio"] }
```

`tokio` feature 自动包含 `ntp`。

## 平台支持

`wasm32-unknown-unknown` 视为浏览器 JS/wasm 平台，使用独立实现：`Date.now()` 只用于建立初始时间锚点，后续读取由
`web_time::Instant` 推进。WASI 与其他非 JS/wasm 平台使用原生实现。

`tokio` feature 仅影响非 JS/wasm 平台；在浏览器 wasm 中不会引入 Tokio 网络或运行时实现。

以下 API 在两个平台具有相同的时间读取能力：

- `current_millis() -> Result<i64>`：修正后的 Unix 毫秒时间戳；超出 `i64` 范围时返回错误。
- `current_millis_i128() -> Result<i128>`：修正后的有符号 Unix 毫秒时间戳，可表示 Epoch 前时间。
- `is_ntp() -> bool`：至少成功校时一次后返回 `true`。

```rust
use framework_datetime::{current_millis, current_millis_i128, is_ntp};

let millis: i64 = current_millis()?;
let signed_millis: i128 = current_millis_i128()?;
let calibrated: bool = is_ntp();
# Ok::<(), Box<dyn std::error::Error>>(())
```

`current_millis_i128()` 在锚点建立时读取系统时间：原生平台使用 `SystemTime`（Epoch 前返回负值），
浏览器 wasm 使用 `Date.now()`（非有限数值时报错）。锚点建立后，读取结果由锚点时间加单调时钟经过的毫秒数得到。

## 非 JS/wasm NTP

`NtpConfig` 使用 NTP 服务器列表、请求超时、同步间隔和重试间隔：

| 配置项            | 默认值                    |
|-------------------|---------------------------|
| `servers`         | `NTP_SERVERS`（18 个公共 NTP 服务器） |
| `request_timeout` | 5 秒                      |
| `sync_interval`   | 10 分钟                   |
| `retry_interval`  | 5 秒                      |

`NTP_SERVERS` 同时作为公开常量导出，默认值为 `time.cloudflare.com`、`time.google.com`、`pool.ntp.org`、
`time.windows.com`、`time.nist.gov`、`time.apple.com`、`time.asia.apple.com`、`cn.ntp.org.cn`、
`ntp.ntsc.ac.cn`、`cn.pool.ntp.org` 及阿里云 `ntp.aliyun.com`、`ntp1.aliyun.com` 至 `ntp7.aliyun.com`。

```rust
use framework_datetime::{NtpConfig, init_ntp};

// 使用内置服务器列表
init_ntp(NtpConfig::default())?;

// 自定义服务器列表
let config = NtpConfig {
    servers: vec!["ntp.aliyun.com".to_string(), "cn.pool.ntp.org".to_string()],
    ..NtpConfig::default()
};
init_ntp(config)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

配置校验：`servers` 不能为空，`request_timeout`、`sync_interval`、`retry_interval` 都必须大于零，
否则 `init_ntp` 返回错误。

未启用 `tokio` 时，NTP 查询使用标准库线程并发执行；启用 `tokio` 时，查询由专用后台线程中的 Tokio runtime
管理。初始化只启动一个常驻任务：重复调用 `init_ntp` 直接返回 `Ok(())`，不会替换首个任务；
首次启动失败时会重置初始化标记，允许再次调用重试。

每轮同步会同时向 `servers` 中的全部服务器发起查询，采用最先成功返回的结果（Tokio 实现会中止其余任务，
标准库实现返回后其余线程的结果被丢弃），全部失败则等待 `retry_interval` 后重试；同步成功后等待
`sync_interval` 再进入下一轮。写入新锚点失败时同样按 `retry_interval` 重试。

校时值由**本地系统时间加 NTP 时钟偏移**得到，因此本地系统时间本身偏离过大时，校时结果也会随之偏离。
校时成功后锚点被替换，`is_ntp()` 开始返回 `true`，`wait_ntp()` 的等待者会被唤醒。

## 浏览器 wasm NTP

浏览器 wasm 不直接访问 NTP 服务器。启用 `ntp` 后，`NtpConfig` 仅包含 `sync_interval`（默认 10 分钟），
`init_ntp` 必须同时接收配置和同步时间提供函数：

```rust
use anyhow::Result;
use framework_datetime::{NtpConfig, init_ntp};
use std::time::Duration;

let config = NtpConfig {
    sync_interval: Duration::from_secs(10 * 60),
};

init_ntp(config, || -> Result<u128> {
    // 返回已由外部服务校准的 Unix 毫秒绝对时间戳。
    Ok(1_735_689_600_000)
})?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

回调类型为 `Fn() -> anyhow::Result<u128> + 'static`。其返回值必须是 Unix Epoch 起算的 **毫秒绝对时间戳**，不是 NTP
偏移量、秒或纳秒；值超过 `i128::MAX` 时视为该次校时失败。

初始化会建立浏览器 `setInterval` 定时器并立即调用一次回调。回调失败不会停止定时器，后续周期会自动重试；首次成功前
`is_ntp()` 返回 `false`，`wait_ntp()` 会持续等待。重复初始化不会替换首个定时器或回调，页面生命周期内不提供停止接口；
`setInterval` 创建失败时会重置初始化标记并返回错误，可再次调用重试。

`sync_interval` 必须至少为 1 毫秒，且不能超过 `2_147_483_647` 毫秒。浏览器中的网络请求通常是异步的，调用方应在回调执行前完成请求并缓存最近一次可用的校准时间。

## 等待首次校时

启用 `ntp` 后，两个平台均可等待首次成功校时：

```rust
use framework_datetime::wait_ntp;

wait_ntp().await;
```

该 Future 不绑定 Tokio，可由任意 Rust async runtime 驱动。未初始化或尚未成功校时时，它不会自行超时，调用方应按需增加超时控制。

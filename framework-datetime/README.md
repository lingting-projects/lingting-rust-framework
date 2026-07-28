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

- `current_millis() -> Result<u64>`：修正后的 Unix 毫秒时间戳；Epoch 前时间或超出 `u64` 范围时返回错误。
- `current_millis_u128() -> Result<u128>`：修正后的非负 Unix 毫秒时间戳；Epoch 前时间时返回错误。
- `current_millis_i128() -> Result<i128>`：修正后的有符号 Unix 毫秒时间戳，可表示 Epoch 前时间。
- `is_ntp() -> bool`：至少成功校时一次后返回 `true`。

```rust
use framework_datetime::{current_millis, current_millis_i128, current_millis_u128, is_ntp};

let millis: u64 = current_millis()?;
let signed_millis: i128 = current_millis_i128()?;
let unsigned_millis: u128 = current_millis_u128()?;
let calibrated: bool = is_ntp();
# Ok::<(), Box<dyn std::error::Error>>(())
```

## 非 JS/wasm NTP

非 JS/wasm 平台的 NTP 行为保持不变。`NtpConfig` 使用 NTP 服务器列表、请求超时、同步间隔和重试间隔：

| 配置项            | 默认值  |
|-------------------|---------|
| `request_timeout` | 5 秒    |
| `sync_interval`   | 10 分钟 |
| `retry_interval`  | 5 秒    |

```rust
use framework_datetime::{NtpConfig, init_ntp};

init_ntp(NtpConfig::default())?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

未启用 `tokio` 时，NTP 查询使用标准库线程并发执行；启用 `tokio` 时，查询由专用后台线程中的 Tokio runtime
管理。初始化只启动一个常驻任务，NTP 失败会按重试间隔继续尝试。

## 浏览器 wasm NTP

浏览器 wasm 不直接访问 NTP 服务器。启用 `ntp` 后，`NtpConfig` 仅包含 `sync_interval`，`init_ntp` 必须同时接收配置和同步时间提供函数：

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
`is_ntp()` 返回 `false`，`wait_ntp()` 会持续等待。重复初始化不会替换首个定时器或回调，页面生命周期内不提供停止接口。

`sync_interval` 必须至少为 1 毫秒，且不能超过 `2_147_483_647` 毫秒。浏览器中的网络请求通常是异步的，调用方应在回调执行前完成请求并缓存最近一次可用的校准时间。

## 等待首次校时

启用 `ntp` 后，两个平台均可等待首次成功校时：

```rust
use framework_datetime::wait_ntp;

wait_ntp().await;
```

该 Future 不绑定 Tokio，可由任意 Rust async runtime 驱动。未初始化或尚未成功校时时，它不会自行超时，调用方应按需增加超时控制。

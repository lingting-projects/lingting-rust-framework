# framework-datetime

提供 Unix 毫秒时间戳，并可选地通过 NTP 持续校时。

内部时钟以 `i128` Unix 毫秒时间戳和 `Instant` 单调时钟组成不可变锚点。程序启动后，常规时间读取不受操作系统时钟回拨影响；NTP
校时成功后会以校准后的真实时间替换锚点，校时结果允许向前或向后调整，以时间准确性为优先。

## 安装

默认仅提供本地时间读取：

```toml
[dependencies]
framework-datetime = { path = "../framework-datetime" }
```

启用 NTP 校时：

```toml
[dependencies]
framework-datetime = { path = "../framework-datetime", features = ["ntp"] }
```

启用 Tokio 管理 NTP 并发查询：

```toml
[dependencies]
framework-datetime = { path = "../framework-datetime", features = ["tokio"] }
```

`tokio` feature 自动包含 `ntp`。不启用 `tokio` 时，NTP 查询使用标准库线程并发执行。

## 时间读取

```rust
use framework_datetime::{current_millis, current_millis_i128, current_millis_u128, is_ntp};

let millis: u64 = current_millis() ?;
let signed_millis: i128 = current_millis_i128() ?;
let unsigned_millis: u128 = current_millis_u128() ?;
let calibrated: bool = is_ntp();
# Ok::<(), Box<dyn std::error::Error> > (())
```

`current_millis()` 是默认接口，返回 `Result<u64>`。当时间早于 Unix Epoch 或无法转换为 `u64` 时返回错误；需要保留 Epoch
前时间时使用 `current_millis_i128()`。

`is_ntp()` 在至少一次 NTP 校时成功后返回 `true`。未启用 `ntp`、未启动校时任务或尚未成功校时时返回 `false`。

## NTP 校时

`NtpConfig::default()` 使用公开的 `NTP_SERVERS` 服务列表，并设置以下默认参数：

| 配置项            | 默认值  |
|-------------------|---------|
| `request_timeout` | 5 秒    |
| `sync_interval`   | 10 分钟 |
| `retry_interval`  | 5 秒    |

启动默认校时任务：

```rust
use framework_datetime::{NtpConfig, init_ntp};

init_ntp(NtpConfig::default ()) ?;
# Ok::<(), Box<dyn std::error::Error> > (())
```

指定外部 NTP 服务与刷新策略：

```rust
use framework_datetime::{NtpConfig, init_ntp};
use std::time::Duration;

let config = NtpConfig {
servers: vec![
    "ntp.example.com".to_owned(),
    "time.example.com".to_owned(),
],
request_timeout: Duration::from_secs(3),
sync_interval: Duration::from_secs(15 * 60),
retry_interval: Duration::from_secs(10),
};

init_ntp(config) ?;
# Ok::<(), Box<dyn std::error::Error> > (())
```

`init_ntp` 立即返回并只启动一个后台校时任务。每轮并发请求全部配置服务，首个成功结果即用于重置时钟锚点。启用 `tokio` feature
时，查询任务由专用后台线程中的 Tokio runtime 管理，首个成功结果会取消同轮剩余任务。

## 等待首次校时

`wait_ntp()` 可被任意数量的异步任务同时等待：

```rust
use framework_datetime::{NtpConfig, init_ntp, wait_ntp};

init_ntp(NtpConfig::default ()) ?;
wait_ntp().await;

assert!(framework_datetime::is_ntp());
# Ok::<(), Box<dyn std::error::Error> > (())
```

该 Future 不绑定 Tokio，可由任意 Rust async runtime 驱动。首次校时成功时会唤醒全部等待者；已经成功校时后，后续调用会立即完成。若未调用
`init_ntp` 或所有服务持续失败，等待不会自行超时，调用方应在自己的 runtime 中按需增加超时控制。

## 时间语义

- 首次读取时，以系统 Unix 时间初始化内部锚点。
- 后续读取使用 `Instant` 推进锚点时间，不会因操作系统时钟回拨或前跳改变。
- NTP 成功后，立即以校准后的 NTP Unix 时间替换锚点；为保证真实时间准确性，此次替换允许时间回退或前跳。
- 后续 NTP 同步会继续刷新锚点，以降低本地时钟长期漂移的影响。

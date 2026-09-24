# framework-core

提供跨项目复用的基础类型与运行时能力：统一响应、分页类型、雪花 ID、精确金额、应用目录、日志初始化、多值映射，
并重导出 `framework-datetime` 的全部时间接口。

## 安装

```toml
[dependencies]
framework-core = { path = "../framework-core" }
```

## 模块概览

| 路径 | 内容 |
|------|------|
| `r` | `R<D>` 统一响应与其构造方法 |
| `types` | `RCodeKind`、分页类型、通用 PO/VO 与回调别名 |
| `logging` | `LoggingConfig`、`LogArchiveConfig`、`LoggingGuard`、`LogStrFilter`、`LogDebugFilter`、`init` |
| 根导出 | `Snowflake`、`next_id`、`Money`、`MultiStringValue`、`ApplicationDirectory` |
| 重导出 | `framework_datetime::*`，如 `current_millis`、`wait_ntp` |

## 统一响应

`R<D>` 的字段为 `code`、`message`、`data`，序列化字段名为 camelCase。

```rust
use framework_core::r::R;

let ok = R::ok(1_i32);          // code = 200
let none = R::ok_none();        // data 为 None
let failed = R::failed("出错了"); // code = 500

let from_result: R<i32> = Result::<i32, std::io::Error>::Ok(1).into();
```

`RCodeKind` 提供业务状态码，`code()` 由 `#[auto_enum_impl]` 导出为枚举运行时字段：

| 变体 | `code()` | 序列化 / `Display` |
|------|----------|--------------------|
| `Success` | 200 | `SUCCESS` |
| `Parameter` | 400 | `PARAMETER` |
| `Unauthorized` | 401 | `UNAUTHORIZED` |
| `Forbidden` | 403 | `FORBIDDEN` |
| `Internal` | 500 | `INTERNAL` |

常量 `R_CODE_SUCCESS`、`R_MESSAGE_SUCCESS` 与 `RCodeKind::Success` 对应。

## 通用类型

- `PaginationSort { field, desc }`：排序字段与方向。
- `PaginationParams { current, size, sorts }`：分页请求参数。
- `PaginationResult<D> { total, records }`：分页返回结果。
- `IdPO { id }`、`IdsPO { ids }`、`TimeMillisVO { millis }`：常用入参与出参。
- `FnCallback`：`Arc<dyn Fn() + Send + Sync>`。

上述类型均由 `#[auto_type]` 声明，自动获得 serde 与 specta 派生、camelCase 字段命名，
并在开启 `collect` 时注册类型元数据。

## 雪花 ID

`Snowflake::new(datacenter, worker)` 使用默认纪元 `2025-01-01T00:00:00Z`，
`new_with_epoch` 可自定义。`datacenter` 与 `worker` 各取低 5 位，序号 12 位。

```rust
use framework_core::{Snowflake, next_id};

let snowflake = Snowflake::new(1, 1);
let id: i64 = snowflake.next_id()?;

// 全局实例，节点为 datacenter = 1, worker = 1
let id: i64 = next_id()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

同一毫秒内序号耗尽时逻辑时间自增，因此返回值可能略超前于真实时间。

## 金额

`Money` 以分为最小单位存储 `i64`，序列化与 `Display` 均为元字符串（如 `1.23`），
specta 导出类型为 `String`。

```rust
use framework_core::Money;

let money = Money::from_cents(123);
assert_eq!(money.cents(), 123);
assert_eq!(money.to_string(), "1.23");

let parsed = Money::from_yuan_str("-0.05")?;
assert_eq!(parsed.cents(), -5);

let sum = Money::from_cents(100) + Money::from_cents(23);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`from_yuan_str` 接受可选符号、最多两位小数，格式无效时返回 `MoneyParseError`；支持 `Add`、`Sub`、
`AddAssign`、`SubAssign`、`Neg` 与 `checked_add`、`checked_sub`。

## 应用目录

`ApplicationDirectory` 创建并持有应用运行目录：

- `ApplicationDirectory::root(id)`：系统公共目录，Windows 取 `ALLUSERSPROFILE`，
  Linux 取 `/usr/local/share`，macOS 取 `/Library/Application Support`。
- `ApplicationDirectory::normal(id, parent)`：当前用户目录（`USERPROFILE` / `HOME`）下的 `parent/id`。

debug 构建统一使用 `runtime` 目录（可执行文件同级；`examples` 场景取 `target` 下的 profile 目录），便于本地开发；
release 构建使用上述系统或用户目录。

目录结构：`data`、`cache`、`tmp`、`tmp/logs` 会自动创建；`startup` 为进程启动目录，
`install` 为可执行文件所在目录。

## 日志

```rust
use framework_core::logging::{LoggingConfig, init};

let mut config = LoggingConfig::new();
config.directory = Some("/var/log/my-app".into());
config.combined = true;

let guard = init(&config)?;
// guard 必须存活到进程退出，否则写入器与归档线程会被提前释放
# Ok::<(), std::io::Error>(())
```

`LoggingConfig::new()` 的默认值：`level` 为 `INFO`、`console` 为 `true`、`directory` 为 `None`、
`combined` 为 `false`、`archive` 为保留 7 天。

指定 `directory` 后会按级别写入 `error.log`、`warn.log`、`info.log`、`debug.log`、`trace.log`，
`combined` 为 `true` 时额外写入 `combined.log`。日志按本地日期切分，旧文件压缩为
`archive/YYYY-MM-DD-<name>.gz`；归档线程每小时清理一次超出 `retention_days` 的文件。
空日志文件不生成归档。

日志层会过滤无意义的事件：target 为 `sqlx::query` 时，由 `record_str_filters` 与 `record_debug_filters`
中的过滤函数判定，任一函数返回 `true` 即忽略该事件。

`LoggingConfig` 的默认过滤器：`db.statement` / `message` 包含 `lib_queue`，或 `summary` 等于 `COMMIT`。

```rust
use framework_core::logging::{LoggingConfig, init};
use std::sync::Arc;

let mut config = LoggingConfig::new();
config.push_str_filter(Arc::new(|field, value| {
    field.name() == "db.statement" && value.contains("noisy_table")
}));
config.push_debug_filter(Arc::new(|field, value| {
    field.name() == "summary" && format!("{value:?}") == "BEGIN"
}));

let guard = init(&config)?;
# Ok::<(), std::io::Error>(())
```

过滤函数类型为 `LogStrFilter` 与 `LogDebugFilter`，即
`Arc<dyn Fn(&tracing::field::Field, &str) -> bool + Send + Sync>` 与其 `Debug` 版本；
两个字段本身也是公开的，可直接读写。

## 多值映射

`MultiStringValue` 用于请求头与查询参数，一个键可对应多个值。

```rust
use framework_core::MultiStringValue;
use std::collections::HashMap;

let mut values = MultiStringValue::create(true, HashMap::new()); // true 表示键名转小写
values.set("Content-Type", "application/json; charset=utf-8");

assert_eq!(values.get_first("content-type").unwrap(), "application/json; charset=utf-8");
assert_eq!(values.mime_type().unwrap(), "application/json");
```

序列化输出为 `{ "lower": ..., "map": ... }`，且 `map` 的键有序。

## 时间

本 crate 重导出 `framework-datetime`，`current_millis`、`current_millis_i128`、`is_ntp`、`wait_ntp`
等接口可直接从 `framework_core` 引入。时间能力、NTP 校时与平台差异详见
[framework-datetime/README.md](../framework-datetime/README.md)。

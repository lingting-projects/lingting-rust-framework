//! 整个 crate 通用的默认配置。

use std::time::Duration;

/// 默认请求超时时间。
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// 默认最大重定向次数。
pub const DEFAULT_REDIRECT_LIMIT: i64 = 10;

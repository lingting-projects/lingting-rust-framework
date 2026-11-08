//! 整个 crate 通用的配置常量。

use std::time::Duration;

/// 请求超时时间，两个后端共用。
pub(crate) const TIMEOUT: Duration = Duration::from_secs(60);

/// 跟随的最大重定向次数，两个后端共用。
pub(crate) const REDIRECT_LIMIT: usize = 10;

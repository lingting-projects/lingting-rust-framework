use std::fmt::{Debug, Write as _};
use std::sync::Arc;

use tracing::field::{Field, Visit};

/// SQL 查询日志的 target 名称。
pub(crate) const TARGET: &str = "sqlx::query";
/// SQL 语句字段名。
const FIELD_STATEMENT: &str = "db.statement";
/// SQL 消息字段名。
const FIELD_MESSAGE: &str = "message";
/// SQL 摘要字段名。
const FIELD_SUMMARY: &str = "summary";
/// 需要忽略的 SQL 语句关键字。
const IGNORED_STATEMENT_KEYWORD: &str = "lib_queue";
/// 需要忽略的 SQL 摘要值。
const IGNORED_SUMMARY_VALUE: &str = "COMMIT";

/// 字符串字段过滤函数：返回 `true` 表示该事件应被忽略。
pub type LogStrFilter = Arc<dyn Fn(&Field, &str) -> bool + Send + Sync>;
/// `Debug` 字段过滤函数：返回 `true` 表示该事件应被忽略。
pub type LogDebugFilter = Arc<dyn Fn(&Field, &dyn Debug) -> bool + Send + Sync>;

/// 默认过滤器：`db.statement` / `message` 字符串包含 `lib_queue` 时忽略。
pub(crate) fn default_str_filter_lib_queue() -> LogStrFilter {
    Arc::new(|field, value| {
        (field.name() == FIELD_STATEMENT || field.name() == FIELD_MESSAGE)
            && value.contains(IGNORED_STATEMENT_KEYWORD)
    })
}

/// 默认过滤器：`summary` 字符串等于 `COMMIT` 时忽略。
pub(crate) fn default_str_filter_commit() -> LogStrFilter {
    Arc::new(|field, value| field.name() == FIELD_SUMMARY && value == IGNORED_SUMMARY_VALUE)
}

/// 默认过滤器：`db.statement` / `message` 的 `Debug` 输出包含 `lib_queue` 时忽略。
pub(crate) fn default_debug_filter_lib_queue() -> LogDebugFilter {
    Arc::new(|field, value| {
        if field.name() != FIELD_STATEMENT && field.name() != FIELD_MESSAGE {
            return false;
        }
        let mut statement = String::new();
        let _ = write!(statement, "{value:?}");
        statement.contains(IGNORED_STATEMENT_KEYWORD)
    })
}

/// 默认过滤器：`summary` 的 `Debug` 输出等于 `COMMIT` 时忽略。
pub(crate) fn default_debug_filter_commit() -> LogDebugFilter {
    Arc::new(|field, value| {
        if field.name() != FIELD_SUMMARY {
            return false;
        }
        let mut statement = String::new();
        let _ = write!(statement, "{value:?}");
        statement == IGNORED_SUMMARY_VALUE
    })
}

/// 识别无需记录事件的访问器。
pub(crate) struct LoggingVisitor {
    ignore: bool,
    str_filters: Vec<LogStrFilter>,
    debug_filters: Vec<LogDebugFilter>,
}

impl LoggingVisitor {
    /// 使用给定的过滤器集合创建访问器。
    pub(crate) fn new(str_filters: Vec<LogStrFilter>, debug_filters: Vec<LogDebugFilter>) -> Self {
        Self {
            ignore: false,
            str_filters,
            debug_filters,
        }
    }

    /// 该事件是否无需记录。
    pub(crate) fn is_ignored(&self) -> bool {
        self.ignore
    }
}

impl Visit for LoggingVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if self.ignore {
            return;
        }
        for filter in &self.str_filters {
            if filter(field, value) {
                self.ignore = true;
                return;
            }
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if self.ignore {
            return;
        }
        for filter in &self.debug_filters {
            if filter(field, value) {
                self.ignore = true;
                return;
            }
        }
    }
}

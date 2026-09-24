use std::fmt::Write as _;

use tracing::field::{Field, Visit};
use tracing::{Event, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, Filter};
use tracing_subscriber::registry::LookupSpan;

/// SQL 查询日志的 target 名称。
const TARGET: &str = "sqlx::query";
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

/// 默认的 SQL 日志过滤器。
///
/// 仅处理 target 为 `sqlx::query` 的事件，其余事件一律放行；命中以下任一规则时返回 `false` 忽略该事件：
///
/// - `db.statement` 或 `message` 包含 `lib_queue`。
/// - `summary` 等于 `COMMIT`。
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultSqlxFilter;

impl<S> Filter<S> for DefaultSqlxFilter
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn enabled(&self, _: &Metadata<'_>, _: &Context<'_, S>) -> bool {
        true
    }

    fn event_enabled(&self, event: &Event<'_>, _: &Context<'_, S>) -> bool {
        if event.metadata().target() != TARGET {
            return true;
        }

        let mut visitor = SqlxStatementVisitor::default();
        event.record(&mut visitor);
        !visitor.ignore
    }
}

/// 识别无需记录的 SQL 语句事件的访问器。
#[derive(Default)]
struct SqlxStatementVisitor {
    ignore: bool,
}

impl Visit for SqlxStatementVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if self.ignore {
            return;
        }
        if field.name() == FIELD_STATEMENT || field.name() == FIELD_MESSAGE {
            self.ignore = value.contains(IGNORED_STATEMENT_KEYWORD);
        }
        if field.name() == FIELD_SUMMARY && value == IGNORED_SUMMARY_VALUE {
            self.ignore = true;
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if self.ignore {
            return;
        }
        let keyword = if field.name() == FIELD_SUMMARY {
            IGNORED_SUMMARY_VALUE
        } else if field.name() == FIELD_STATEMENT || field.name() == FIELD_MESSAGE {
            IGNORED_STATEMENT_KEYWORD
        } else {
            return;
        };

        let mut statement = String::new();
        let _ = write!(statement, "{value:?}");
        self.ignore = statement.contains(keyword);
    }
}

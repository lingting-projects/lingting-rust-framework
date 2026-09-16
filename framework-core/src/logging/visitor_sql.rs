use std::fmt::Write as _;

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

/// 识别无需记录的 SQL 语句事件的访问器。
#[derive(Default)]
pub(crate) struct SqlStatementVisitor {
    ignore: bool,
}

impl SqlStatementVisitor {
    /// 该 SQL 事件是否无需记录。
    pub(crate) fn is_ignored(&self) -> bool {
        self.ignore
    }
}

impl Visit for SqlStatementVisitor {
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

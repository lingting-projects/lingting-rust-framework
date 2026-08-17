use crate::{auto_enum, auto_type};
use std::sync::Arc;

pub const R_CODE_SUCCCESS: u32 = 200;
pub const R_MESSAGE_SUCCCESS: &str = "Success";

#[auto_enum]
pub enum RCodeKind {
    Success,
    Parameter,
    Unauthorized,
    Forbidden,
    Internal,
}

#[auto_type]
pub struct R<D> {
    pub code: u32,
    pub message: String,
    pub data: Option<D>,
}

#[auto_type]
pub struct PaginationSort {
    pub field: String,
    pub desc: bool,
}

#[auto_type]
pub struct PaginationParams {
    #[specta(type = i32)]
    pub current: i64,
    #[specta(type = i32)]
    pub size: i64,
    pub sorts: Vec<PaginationSort>,
}

#[auto_type]
pub struct PaginationResult<D> {
    #[specta(type = i32)]
    pub total: i64,
    pub records: Vec<D>,
}

#[auto_type]
pub struct IdPO {
    pub id: i64,
}

#[auto_type]
pub struct IdsPO {
    pub ids: Vec<i64>,
}

#[auto_type]
pub struct TimeMillisVO {
    pub millis: i64,
}

pub type FnCallback = Arc<dyn Fn() + Send + Sync>;

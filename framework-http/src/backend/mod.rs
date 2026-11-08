//! 后端实现：编译期在 wreq 与 reqwest 之间二选一。
//!
//! 两个后端导出同一组条目（`Client`、`Method`、`FetchStream`、`create`、`shared`、
//! `send`、`stream`），crate 其余部分只通过本模块引用，不感知差异。
//! 启用 `wreq` 时使用 wreq；否则使用 reqwest，此时还需启用 `reqwest` feature。

#[cfg(not(feature = "wreq"))]
mod reqwest_backend;
#[cfg(feature = "wreq")]
mod wreq_backend;

#[cfg(not(feature = "wreq"))]
pub(crate) use reqwest_backend::*;
#[cfg(feature = "wreq")]
pub(crate) use wreq_backend::*;

use crate::WebError;
use anyhow::Result;
use futures_util::FutureExt;
use std::future::Future;
use std::panic::AssertUnwindSafe;

pub async fn catch_panic<F, T>(future: F) -> Result<T>
where
    F: Future<Output = T>,
{
    AssertUnwindSafe(future)
        .catch_unwind()
        .await
        .map_err(|payload| WebError::panic(panic_message(payload)).into())
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return format!("请求执行发生 panic：{message}");
    }
    if let Some(message) = payload.downcast_ref::<&str>() {
        return format!("请求执行发生 panic：{message}");
    }
    "请求执行发生未知 panic".to_string()
}

use crate::{WebContext, WebResponse, WebRoute};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// 拦截结果：拦截器对本次请求的处理决定。
pub enum WebInterceptResult {
    /// 继续：不改变后续流程，交由下一个拦截器与路由继续处理。
    Next,
    /// 提前返回：直接以该响应结束请求，不再执行后续拦截器与路由。
    Respond(WebResponse),
    /// 重定向：后续拦截器与路由改用该路由，`None` 表示清空当前路由。
    Redirect(Option<Arc<WebRoute>>),
}

/// Web 拦截器：在路由查找之后、路由执行之前介入请求处理。
#[async_trait]
pub trait WebInterceptor: Send + Sync {
    /// 排序值，按升序执行。
    fn order(&self) -> i32;

    /// 是否对本次请求执行拦截，返回 `false` 时不调用 `intercept`。
    fn should_intercept(&self, context: Arc<WebContext>, route: Option<Arc<WebRoute>>) -> bool;

    /// 执行拦截，返回值决定后续流程。
    async fn intercept(
        &self,
        context: Arc<WebContext>,
        route: Option<Arc<WebRoute>>,
    ) -> Result<WebInterceptResult>;
}

/// 拦截器集合：持有拦截器并提供统一的执行逻辑。
pub struct WebInterceptors {
    interceptors: Vec<Arc<dyn WebInterceptor>>,
}

impl WebInterceptors {
    /// 创建拦截器集合，按 `order()` 升序排序，`order()` 相同的保持传入顺序。
    pub fn new(mut interceptors: Vec<Arc<dyn WebInterceptor>>) -> Self {
        interceptors.sort_by_key(|interceptor| interceptor.order());
        Self { interceptors }
    }

    /// 依次执行拦截器，入参与 `intercept` 一致。
    ///
    /// - `Respond`：立即结束请求，不再执行后续拦截器。
    /// - `Redirect`：后续拦截器与调用方都使用该路由。
    /// - `Next`：没有拦截器改变路由，调用方继续使用传入的路由。
    ///
    /// 拦截器内部重定向后最终返回 `Redirect`，其中携带最后一个拦截器确定的路由。
    pub async fn execute(
        &self,
        context: Arc<WebContext>,
        route: Option<Arc<WebRoute>>,
    ) -> Result<WebInterceptResult> {
        let mut current = route;
        let mut redirected = false;
        for interceptor in &self.interceptors {
            if !interceptor.should_intercept(Arc::clone(&context), current.clone()) {
                continue;
            }
            let result = interceptor
                .intercept(Arc::clone(&context), current.clone())
                .await?;
            match result {
                WebInterceptResult::Next => {}
                WebInterceptResult::Respond(response) => {
                    return Ok(WebInterceptResult::Respond(response));
                }
                WebInterceptResult::Redirect(next) => {
                    current = next;
                    redirected = true;
                }
            }
        }

        if redirected {
            Ok(WebInterceptResult::Redirect(current))
        } else {
            Ok(WebInterceptResult::Next)
        }
    }
}

mod context;
mod cors;
mod dispatch;
mod router;
mod server;
mod shutdown;
mod status;

pub use context::{AxumContext, scope_axum, use_axum};
pub use cors::AxumCors;
pub use router::{WebRouteResultFuture, WebRouteWrapper, WebRouter};
pub use server::{AxumBuilder, AxumServer, axum_builder};
pub use shutdown::AxumShutdown;
pub(crate) use status::AxumShared;
pub use status::AxumStatus;

use crate::dispatch::{AxumState, dispatch};
use axum::Router;
use axum::routing::any;
use framework_web::{WebContext, WebInterceptor};
use std::sync::Arc;

/// CORS 解析函数：按请求上下文返回本次响应的 CORS 配置。
pub type AxumCorsFn = Arc<dyn Fn(&WebContext) -> AxumCors + Send + Sync>;

/// 构建 WebRouter：内部创建 WebRouter，并挂载统一的 fallback 分发。
pub fn build_router(
    wrapper: Option<WebRouteWrapper>,
    interceptors: Vec<Arc<dyn WebInterceptor>>,
) -> WebRouter {
    WebRouter::new(wrapper, interceptors)
}

/// 使用已有 WebRouter 组装 axum Router。
pub(crate) fn build_router_with(
    router: Arc<WebRouter>,
    context: AxumContext,
    cors: AxumCorsFn,
) -> Router {
    Router::new().fallback(any(dispatch)).with_state(AxumState {
        router,
        context,
        cors,
    })
}

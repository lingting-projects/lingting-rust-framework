use crate::AxumContext;
use crate::context::scope_axum;
use framework_web::{
    WebContext, WebError, WebMethod, WebResponse, WebRoute, scope_web, use_web, web_api_iter,
};
use futures_util::future::BoxFuture;
use std::collections::HashMap as Map;
use std::collections::hash_map::Entry;
use std::sync::Arc;

/// 包裹路由执行后的异步结果。
pub type WebRouteResultFuture = BoxFuture<'static, anyhow::Result<WebResponse>>;

/// 路由包装器：在 find 命中 WebRoute 后包裹其运行，用于授权校验、上下文注入等扩展。
pub type WebRouteWrapper = Arc<dyn Fn(Arc<WebRoute>) -> WebRouteResultFuture + Send + Sync>;

/// 默认包装器：直接执行 route.invoke()。
fn default_wrapper() -> WebRouteWrapper {
    Arc::new(|route| Box::pin(async move { Ok(route.invoke().await) }))
}

/// Web 路由：收集所有已注册的 WebRoute，并按方法与路径查找。
pub struct WebRouter {
    routes: Map<WebMethod, Map<String, Arc<WebRoute>>>,
    wrapper: WebRouteWrapper,
}

impl WebRouter {
    pub fn new(wrapper: Option<WebRouteWrapper>) -> Self {
        let mut routes: Map<WebMethod, Map<String, Arc<WebRoute>>> = Map::new();
        for route in web_api_iter() {
            let route = Arc::new(route);
            match routes.entry(route.method.clone()) {
                Entry::Occupied(mut methods) => {
                    methods.get_mut().insert(route.path.clone(), route);
                }
                Entry::Vacant(methods) => {
                    methods.insert(Map::from([(route.path.clone(), route)]));
                }
            }
        }
        Self {
            routes,
            wrapper: wrapper.unwrap_or_else(default_wrapper),
        }
    }

    /// 执行当前请求：注入上下文、查找路由、交由包装器运行。
    pub async fn invoke(
        &self,
        web_context: WebContext,
        axum_context: AxumContext,
    ) -> anyhow::Result<WebResponse> {
        let web_context = Arc::new(web_context);
        let axum_context = Arc::new(axum_context);
        scope_axum(
            axum_context,
            scope_web(web_context, async {
                let route = self.find().await?;
                (self.wrapper)(route).await
            }),
        )
        .await
    }

    /// 按当前请求的方法与路径查找路由。
    async fn find(&self) -> anyhow::Result<Arc<WebRoute>> {
        let context = use_web()?;
        let request = context.request();
        let path = request.path.trim_matches('/');
        self.routes
            .get(&request.method)
            .and_then(|routes| routes.get(path))
            .cloned()
            .ok_or_else(|| WebError::not_found("请求接口不存在").into())
    }
}

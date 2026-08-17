use crate::{AuthRule, WebMethod, WebResponse};
use futures_util::future::BoxFuture;
use std::sync::Arc;

pub type WebRouteFuture = BoxFuture<'static, WebResponse>;
pub type WebRouteInvoke = Arc<dyn Fn() -> WebRouteFuture + Send + Sync>;

pub struct WebRoute {
    pub method: WebMethod,
    pub path: String,
    pub auth: AuthRule,
    pub invoke: WebRouteInvoke,
}

impl WebRoute {
    pub async fn invoke(&self) -> WebResponse {
        (self.invoke)().await
    }
}

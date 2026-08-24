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

#[cfg(feature = "collect")]
#[doc(hidden)]
pub struct WebApiBuilder(pub fn() -> WebRoute);

#[cfg(feature = "collect")]
inventory::collect!(WebApiBuilder);

impl WebRoute {
    pub async fn invoke(&self) -> WebResponse {
        (self.invoke)().await
    }
}

#[cfg(feature = "collect")]
pub fn web_api_iter() -> Box<dyn Iterator<Item = WebRoute>> {
    Box::new(
        inventory::iter::<WebApiBuilder>
            .into_iter()
            .map(|builder| (builder.0)()),
    )
}

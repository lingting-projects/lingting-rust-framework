mod auth;
mod context;
mod error;
mod from_web;
mod panic;
mod request;
mod response;
mod route;

pub use auth::AuthRule;
pub use context::{WebContext, scope_web, use_web};
pub use error::{WebError, WebErrorKind};
pub use framework_proc_web::{
    web_api, web_api_delete, web_api_get, web_api_patch, web_api_post, web_api_put,
};
pub use from_web::{FromWeb, Json, Query};
pub use panic::catch_panic;
pub use request::{WebMethod, WebRequest};
pub use response::WebResponse;
pub use route::{WebRoute, WebRouteFuture};

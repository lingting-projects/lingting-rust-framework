use crate::{AxumContext, AxumCors, AxumCorsFn, WebRouteWrapper, WebRouter, build_router_with};
use anyhow::{Context, Result, anyhow};
use framework_web::WebContext;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::net::TcpListener;

/// 创建 Axum 服务构造器，`bind_port` 小于等于 0 时使用随机端口。
pub fn axum_builder(bind_address: impl Into<String>, bind_port: i32) -> AxumBuilder {
    AxumBuilder {
        bind_address: bind_address.into(),
        bind_port: u16::try_from(bind_port).unwrap_or_default(),
        cors: default_cors(),
    }
}

/// Axum 服务构造器。
pub struct AxumBuilder {
    bind_address: String,
    bind_port: u16,
    cors: AxumCorsFn,
}

impl AxumBuilder {
    /// 设置 CORS 解析函数，默认固定返回拒绝跨域的懒加载值。
    pub fn with_cors<F>(mut self, cors: F) -> Self
    where
        F: Fn(&WebContext) -> AxumCors + Send + Sync + 'static,
    {
        self.cors = Arc::new(cors);
        self
    }

    /// 绑定端口。
    pub async fn bind(self) -> Result<AxumServer> {
        let address = format!("{}:{}", self.bind_address, self.bind_port);
        let listener = TcpListener::bind(&address)
            .await
            .with_context(|| format!("监听地址 {address} 失败"))?;
        let local = listener
            .local_addr()
            .with_context(|| format!("读取监听地址 {address} 失败"))?;
        Ok(AxumServer {
            listener: Mutex::new(Some(listener)),
            context: AxumContext::new(self.bind_address, local.port()),
            cors: self.cors,
            router: OnceLock::new(),
        })
    }
}

/// Axum 服务。
pub struct AxumServer {
    listener: Mutex<Option<TcpListener>>,
    context: AxumContext,
    cors: AxumCorsFn,
    router: OnceLock<Arc<WebRouter>>,
}

impl AxumServer {
    /// 绑定地址与实际端口。
    pub fn context(&self) -> &AxumContext {
        &self.context
    }

    /// 服务启动时创建的 Web 路由实例，未启动时为 `None`。
    pub fn router(&self) -> Option<Arc<WebRouter>> {
        self.router.get().cloned()
    }

    /// 启动服务并持续运行。
    pub async fn run(&self, wrapper: Option<WebRouteWrapper>) -> Result<()> {
        let listener = self
            .listener
            .lock()
            .map_err(|error| anyhow!("Axum 监听器锁定失败：{error}"))?
            .take()
            .ok_or_else(|| anyhow!("Axum 服务已启动"))?;
        let router = Arc::new(WebRouter::new(wrapper));
        let app = build_router_with(
            Arc::clone(&router),
            self.context.clone(),
            Arc::clone(&self.cors),
        );
        let _ = self.router.set(router);
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .context("Axum 服务运行失败")
    }
}

fn default_cors() -> AxumCorsFn {
    Arc::new(|_| AxumCors::fixed().clone())
}

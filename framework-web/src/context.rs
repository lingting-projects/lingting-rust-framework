use crate::{WebError, WebRequest};
use anyhow::{Result, anyhow};
use bytes::Bytes;
use framework_core::MultiStringValue;
use serde_json::{Map, Value};
use std::future::Future;
use std::sync::{Arc, OnceLock};

pub struct WebContext {
    request: Arc<WebRequest>,
    body_json: OnceLock<Value>,
    query_json: OnceLock<Value>,
}

impl WebContext {
    pub fn new(request: Arc<WebRequest>) -> Self {
        Self {
            request,
            body_json: OnceLock::new(),
            query_json: OnceLock::new(),
        }
    }

    pub fn request(&self) -> &WebRequest {
        &self.request
    }

    pub fn request_arc(&self) -> Arc<WebRequest> {
        Arc::clone(&self.request)
    }

    pub fn query(&self) -> &MultiStringValue {
        &self.request.query
    }

    pub fn body(&self) -> Bytes {
        self.request.body.clone()
    }

    pub fn body_json(&self) -> Result<&Value> {
        if let Some(value) = self.body_json.get() {
            return Ok(value);
        }

        let value = serde_json::from_slice::<Value>(&self.request.body)
            .map_err(|error| WebError::parameter("请求体不是有效的 JSON", error))?;
        self.body_json
            .set(value)
            .map_err(|_| anyhow!("请求体 JSON 缓存设置失败"))?;
        self.body_json
            .get()
            .ok_or_else(|| anyhow!("请求体 JSON 缓存读取失败"))
    }

    pub fn query_json(&self) -> Result<&Value> {
        if let Some(value) = self.query_json.get() {
            return Ok(value);
        }

        let mut object = Map::new();
        self.request.query.for_each(|name, values| {
            let value = if values.len() == 1 {
                Value::String(values[0].clone())
            } else {
                Value::Array(
                    values
                        .iter()
                        .map(|item| Value::String(item.clone()))
                        .collect(),
                )
            };
            object.insert(name.clone(), value);
        });
        self.query_json
            .set(Value::Object(object))
            .map_err(|_| anyhow!("查询参数 JSON 缓存设置失败"))?;
        self.query_json
            .get()
            .ok_or_else(|| anyhow!("查询参数 JSON 缓存读取失败"))
    }
}

tokio::task_local! {
    static WEB_CONTEXT: Arc<WebContext>;
}

pub async fn scope_web<F>(context: Arc<WebContext>, future: F) -> F::Output
where
    F: Future,
{
    WEB_CONTEXT.scope(context, future).await
}

pub fn use_web() -> Result<Arc<WebContext>> {
    WEB_CONTEXT
        .try_with(Arc::clone)
        .map_err(|error| anyhow!("当前调用不在 Web 上下文作用域内：{error}"))
}

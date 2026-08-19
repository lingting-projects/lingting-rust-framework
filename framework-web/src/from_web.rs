use crate::{WebError, WebMethod, use_web};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use framework_core::types::{PaginationParams, PaginationSort};
use serde::Deserialize;
use serde::de::DeserializeOwned;

const DEFAULT_PAGE_CURRENT: i64 = 1;
const DEFAULT_PAGE_SIZE: i64 = 10;
const MAX_PAGE_SIZE: i64 = 100;
pub struct Json<T>(pub T);

pub struct Query<T>(pub T);

#[async_trait]
pub trait FromWeb: Sized + Send {
    async fn from_web() -> Result<Self>;
}

#[async_trait]
impl<T> FromWeb for Json<T>
where
    T: DeserializeOwned + Send,
{
    async fn from_web() -> Result<Self> {
        let context = use_web()?;
        let source = if context.request().method == WebMethod::Get {
            context.query_json()?
        } else {
            context.body_json()?
        };
        T::deserialize(source)
            .map(Self)
            .map_err(|error| WebError::parameter("请求参数转换失败", error).into())
    }
}

#[async_trait]
impl<T> FromWeb for Query<T>
where
    T: DeserializeOwned + Send,
{
    async fn from_web() -> Result<Self> {
        let context = use_web()?;
        T::deserialize(context.query_json()?)
            .map(Self)
            .map_err(|error| WebError::parameter("查询参数转换失败", error).into())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaginationInput {
    current: Option<i64>,
    size: Option<i64>,
    #[serde(default)]
    sorts: Vec<PaginationSort>,
}

#[async_trait]
impl FromWeb for PaginationParams {
    async fn from_web() -> Result<Self> {
        let context = use_web()?;
        let source = if context.request().method == WebMethod::Get {
            context.query_json()?
        } else {
            context.body_json()?
        };
        let pagination = PaginationInput::deserialize(source)
            .map_err(|error| WebError::parameter("分页参数转换失败", error))?;
        let size = pagination
            .size
            .filter(|size| *size > 0)
            .unwrap_or(DEFAULT_PAGE_SIZE);
        if size > MAX_PAGE_SIZE {
            return Err(WebError::parameter(
                format!("分页大小不能超过 {MAX_PAGE_SIZE}"),
                anyhow!("分页大小超出上限：{size}"),
            )
            .into());
        }
        if let Some(sort) = pagination
            .sorts
            .iter()
            .find(|sort| !is_safe_sort_field(&sort.field))
        {
            return Err(WebError::parameter(
                "分页排序字段包含非法字符",
                anyhow!("可疑分页排序字段：{}", sort.field),
            )
            .into());
        }
        Ok(Self {
            current: pagination
                .current
                .filter(|current| *current > 0)
                .unwrap_or(DEFAULT_PAGE_CURRENT),
            size,
            sorts: pagination.sorts,
        })
    }
}

fn is_safe_sort_field(field: &str) -> bool {
    let mut chars = field.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

//! 整个 crate 通用的转换与读取工具。
//!
//! 请求头与响应头统一用 [`MultiStringValue`] 表达，与具体后端头部类型互转的部分见
//! `backend::utils`，这里只保留与后端无关的通用部分。

use std::collections::HashMap;

use framework_core::MultiStringValue;

/// 多值映射展开为 (名称, 值) 列表，同名会有多条。
pub(crate) fn pairs(headers: &MultiStringValue) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    headers.for_each(|name, values| {
        for value in values {
            pairs.push((name.clone(), value.clone()));
        }
    });
    pairs
}

/// 由 (名称, 值) 列表构建多值映射，名称统一小写。
pub(crate) fn from_response<I, K, V>(items: I) -> MultiStringValue
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (name, value) in items {
        map.entry(name.as_ref().to_ascii_lowercase())
            .or_default()
            .push(value.as_ref().to_string());
    }

    MultiStringValue::create(true, map)
}

/// 读取响应头，返回第一个值。
pub(crate) fn header<'a>(headers: &'a MultiStringValue, name: &str) -> Option<&'a String> {
    headers.get_first(name)
}

/// 读取 `content-length` 响应头。
pub(crate) fn content_length(headers: &MultiStringValue) -> Option<u64> {
    header(headers, "content-length")?.trim().parse().ok()
}

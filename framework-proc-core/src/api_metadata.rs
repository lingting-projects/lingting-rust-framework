/// API 请求参数的传输位置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiParameterKind {
    Body,
    Query,
}

/// 供 TypeScript API 导出使用的请求参数描述信息。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiParameterMetadata {
    pub name: &'static str,
    pub type_name: &'static str,
    pub kind: ApiParameterKind,
}

/// API 的 TypeScript 返回值描述信息。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiReturnType {
    Void,
    Blob,
    Type(&'static str),
}

/// 供 TypeScript API 导出使用的静态描述信息。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiMetadata {
    pub name: &'static str,
    pub namespace: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub parameters: &'static [ApiParameterMetadata],
    pub return_type: ApiReturnType,
}

#[cfg(feature = "collect")]
inventory::collect!(ApiMetadata);

#[cfg(feature = "collect")]
pub fn api_metadata_iter() -> Box<dyn Iterator<Item = &'static ApiMetadata>> {
    Box::new(inventory::iter::<ApiMetadata>.into_iter())
}

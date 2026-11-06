/// 请求参数的传输位置（TypeScript 与 Rust 代码生成共用）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiParameterKind {
    Body,
    Query,
}

/// 单个目标语言（TypeScript）的类型描述。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TsApiTypeMetadata {
    #[cfg(feature = "ts")]
    pub type_name: Option<&'static str>,
}

/// 单个目标语言（Rust）的类型描述。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RustApiTypeMetadata {
    #[cfg(feature = "rust")]
    pub type_name: Option<&'static str>,
}

/// 请求参数描述信息。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiParameterMetadata {
    pub name: &'static str,
    pub kind: ApiParameterKind,
    #[cfg(feature = "ts")]
    pub ts: TsApiTypeMetadata,
    #[cfg(feature = "rust")]
    pub rust: RustApiTypeMetadata,
}

/// 返回值类别（TypeScript 与 Rust 代码生成共用）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiReturnKind {
    Void,
    Blob,
    Type,
}

/// API 的返回值描述信息。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiReturnType {
    pub kind: ApiReturnKind,
    #[cfg(feature = "ts")]
    pub ts: TsApiTypeMetadata,
    #[cfg(feature = "rust")]
    pub rust: RustApiTypeMetadata,
}

/// 供 TypeScript、Rust API 导出使用的静态描述信息。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiMetadata {
    pub name: &'static str,
    pub namespace: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub parameters: &'static [ApiParameterMetadata],
    pub return_type: ApiReturnType,
}

impl ApiParameterMetadata {
    pub const fn new(
        name: &'static str,
        kind: ApiParameterKind,
        ts_type_name: &'static str,
        rust_type_name: &'static str,
    ) -> Self {
        #[cfg(not(feature = "ts"))]
        let _ = ts_type_name;
        #[cfg(not(feature = "rust"))]
        let _ = rust_type_name;
        Self {
            name,
            kind,
            #[cfg(feature = "ts")]
            ts: TsApiTypeMetadata {
                type_name: Some(ts_type_name),
            },
            #[cfg(feature = "rust")]
            rust: RustApiTypeMetadata {
                type_name: Some(rust_type_name),
            },
        }
    }

    /// 参数类型必填，缺失只会出现在手工构造的元数据里，此时返回 `"unknown"`。
    #[cfg(feature = "ts")]
    pub const fn ts_type_name(self) -> &'static str {
        match self.ts.type_name {
            Some(name) => name,
            None => "unknown",
        }
    }

    /// 参数类型必填，缺失只会出现在手工构造的元数据里，此时返回 `"unknown"`。
    #[cfg(feature = "rust")]
    pub const fn rust_type_name(self) -> &'static str {
        match self.rust.type_name {
            Some(name) => name,
            None => "unknown",
        }
    }
}

impl ApiReturnType {
    pub const fn void() -> Self {
        Self {
            kind: ApiReturnKind::Void,
            #[cfg(feature = "ts")]
            ts: TsApiTypeMetadata { type_name: None },
            #[cfg(feature = "rust")]
            rust: RustApiTypeMetadata { type_name: None },
        }
    }

    pub const fn blob() -> Self {
        Self {
            kind: ApiReturnKind::Blob,
            #[cfg(feature = "ts")]
            ts: TsApiTypeMetadata { type_name: None },
            #[cfg(feature = "rust")]
            rust: RustApiTypeMetadata { type_name: None },
        }
    }

    pub const fn ty(ts_type_name: &'static str, rust_type_name: &'static str) -> Self {
        #[cfg(not(feature = "ts"))]
        let _ = ts_type_name;
        #[cfg(not(feature = "rust"))]
        let _ = rust_type_name;
        Self {
            kind: ApiReturnKind::Type,
            #[cfg(feature = "ts")]
            ts: TsApiTypeMetadata {
                type_name: Some(ts_type_name),
            },
            #[cfg(feature = "rust")]
            rust: RustApiTypeMetadata {
                type_name: Some(rust_type_name),
            },
        }
    }

    #[cfg(feature = "ts")]
    pub const fn ts_type_name(self) -> Option<&'static str> {
        self.ts.type_name
    }

    #[cfg(feature = "rust")]
    pub const fn rust_type_name(self) -> Option<&'static str> {
        self.rust.type_name
    }
}

impl ApiMetadata {
    #[cfg(feature = "ts")]
    pub fn parameters_colon(&self) -> Vec<String> {
        self.parameters
            .iter()
            .map(|parameter| format!("{}: {}", parameter.name, parameter.ts_type_name()))
            .collect::<Vec<_>>()
    }

    pub fn parameter_names(&self, kind: ApiParameterKind) -> Vec<String> {
        self.parameters
            .iter()
            .filter(|parameter| parameter.kind == kind)
            .map(|parameter| parameter.name.to_string())
            .collect::<Vec<_>>()
    }
}

#[cfg(feature = "collect")]
inventory::collect!(ApiMetadata);

#[cfg(feature = "collect")]
pub fn api_metadata_iter() -> Box<dyn Iterator<Item = &'static ApiMetadata>> {
    Box::new(inventory::iter::<ApiMetadata>.into_iter())
}

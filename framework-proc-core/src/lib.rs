mod api_metadata;
mod type_metadata;
mod typescript;

#[cfg(feature = "collect")]
pub use api_metadata::api_metadata_iter;
pub use api_metadata::{ApiMetadata, ApiParameterKind, ApiParameterMetadata, ApiReturnType};
pub use serde_json;
#[cfg(feature = "collect")]
pub use type_metadata::enum_metadata_iter;
#[cfg(feature = "collect")]
pub use type_metadata::type_metadata_iter;
pub use type_metadata::{EnumMetadata, EnumValue, TypeKind, TypeMetadata, enum_field_value};
pub use typescript::*;

#[cfg(feature = "collect")]
#[doc(hidden)]
pub mod __private {
    pub use inventory;
}

/// 向内部类型元数据注册表提交静态类型信息。
#[macro_export]
macro_rules! push_type_metadata {
    ($metadata:expr) => {
        $crate::__private::inventory::submit! {
            $metadata
        }
    };
}

/// 向内部枚举运行时导出元数据注册表提交静态信息。
#[macro_export]
macro_rules! push_enum_metadata {
    ($metadata:expr) => {
        $crate::__private::inventory::submit! {
            $metadata
        }
    };
}

/// 向内部 API 元数据注册表提交静态 API 信息。
#[macro_export]
macro_rules! push_api_metadata {
    ($metadata:expr) => {
        $crate::__private::inventory::submit! {
            $metadata
        }
    };
}

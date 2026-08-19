/// 自动注册类型的类别。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeKind {
    Struct,
    Enum,
}

/// 供 TypeScript 导出和运行时审计使用的类型描述信息。
#[derive(Clone, Copy)]
pub struct TypeMetadata {
    pub kind: TypeKind,
    pub type_name: fn() -> &'static str,
    pub derives: &'static [&'static str],
    pub attributes: &'static [&'static str],
    pub register: Option<fn(specta::Types) -> specta::Types>,
}

/// 枚举 TypeScript 运行时导出需要的单个值。
pub struct EnumValue {
    pub value: String,
    pub fields: Vec<(&'static str, serde_json::Value)>,
}

/// 将枚举字段值转换为 TypeScript 运行时导出所需的 JSON 值。
pub fn enum_field_value(
    value: impl serde::Serialize,
) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::to_value(value)
}

/// 枚举 TypeScript 运行时导出描述信息。
#[derive(Clone, Copy)]
pub struct EnumMetadata {
    pub type_name: fn() -> &'static str,
    pub fields: &'static [&'static str],
    pub values: fn() -> Result<Vec<EnumValue>, serde_json::Error>,
}

impl EnumMetadata {
    pub fn full_name(self) -> &'static str {
        (self.type_name)()
    }

    pub fn values(self) -> Result<Vec<EnumValue>, serde_json::Error> {
        (self.values)()
    }
}

impl TypeMetadata {
    pub fn full_name(self) -> &'static str {
        (self.type_name)()
    }

    pub fn register_type(self, types: specta::Types) -> specta::Types {
        match self.register {
            Some(register) => register(types),
            None => types,
        }
    }
}

#[cfg(feature = "collect")]
inventory::collect!(TypeMetadata);

#[cfg(feature = "collect")]
inventory::collect!(EnumMetadata);

/// 返回当前二进制中已注册的结构体与枚举元数据。
#[cfg(feature = "collect")]
pub fn type_metadata_iter() -> Box<dyn Iterator<Item = &'static TypeMetadata>> {
    Box::new(inventory::iter::<TypeMetadata>.into_iter())
}

/// 返回当前二进制中已注册的枚举运行时导出元数据。
#[cfg(feature = "collect")]
pub fn enum_metadata_iter() -> Box<dyn Iterator<Item = &'static EnumMetadata>> {
    Box::new(inventory::iter::<EnumMetadata>.into_iter())
}

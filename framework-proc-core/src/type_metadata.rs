/// 自动注册类型的类别（TypeScript 与 Rust 代码生成共用）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeKind {
    Struct,
    Enum,
}

/// 类型的公共元数据。
#[derive(Clone, Copy)]
pub struct TypeMetadata {
    pub kind: TypeKind,
    pub type_name: fn() -> &'static str,
    #[cfg(feature = "ts")]
    pub ts: TsTypeMetadata,
    #[cfg(feature = "rust")]
    pub rust: RustTypeMetadata,
}

/// TypeScript 代码生成所需的类型信息。
#[derive(Clone, Copy)]
pub struct TsTypeMetadata {
    /// specta 注册函数；`auto_type` / `auto_enum` 关闭 `specta` 时为 `None`。
    #[cfg(feature = "ts")]
    pub register: Option<fn(specta::Types) -> specta::Types>,
}

/// Rust 代码生成所需的类型信息。
#[derive(Clone, Copy)]
pub struct RustTypeMetadata {
    /// 规范化后的派生项名称，用于生成端的依赖推断与审计。
    #[cfg(feature = "rust")]
    pub derives: &'static [&'static str],
    /// 条目与字段上的属性文本，用于生成端的依赖推断与审计。
    #[cfg(feature = "rust")]
    pub attributes: &'static [&'static str],
    /// 清洗后的类型定义快照（Rust 源码片段）；`None` 时生成端回退为重导出。
    #[cfg(feature = "rust")]
    pub definition: Option<fn() -> &'static str>,
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
    pub const fn new(kind: TypeKind, type_name: fn() -> &'static str) -> Self {
        Self {
            kind,
            type_name,
            #[cfg(feature = "ts")]
            ts: TsTypeMetadata::none(),
            #[cfg(feature = "rust")]
            rust: RustTypeMetadata::none(),
        }
    }

    /// 附加 TypeScript 侧信息；未开启 `ts` 特性时忽略。
    pub const fn with_ts(self, ts: TsTypeMetadata) -> Self {
        #[cfg(feature = "ts")]
        {
            let mut this = self;
            this.ts = ts;
            this
        }
        #[cfg(not(feature = "ts"))]
        {
            let _ = ts;
            self
        }
    }

    /// 附加 Rust 侧信息；未开启 `rust` 特性时忽略。
    pub const fn with_rust(self, rust: RustTypeMetadata) -> Self {
        #[cfg(feature = "rust")]
        {
            let mut this = self;
            this.rust = rust;
            this
        }
        #[cfg(not(feature = "rust"))]
        {
            let _ = rust;
            self
        }
    }

    pub fn full_name(self) -> &'static str {
        (self.type_name)()
    }

    #[cfg(feature = "ts")]
    pub fn register_type(self, types: specta::Types) -> specta::Types {
        self.ts.register_type(types)
    }
}

impl TsTypeMetadata {
    /// 不含任何 TypeScript 侧信息。
    pub const fn none() -> Self {
        Self {
            #[cfg(feature = "ts")]
            register: None,
        }
    }

    /// 使用 specta 注册函数构造；未开启 `ts` 特性时忽略参数。
    pub const fn register(register: fn(specta::Types) -> specta::Types) -> Self {
        #[cfg(feature = "ts")]
        {
            Self {
                register: Some(register),
            }
        }
        #[cfg(not(feature = "ts"))]
        {
            let _ = register;
            Self {}
        }
    }

    #[cfg(feature = "ts")]
    pub fn register_type(self, types: specta::Types) -> specta::Types {
        match self.register {
            Some(register) => register(types),
            None => types,
        }
    }
}

impl RustTypeMetadata {
    /// 不含任何 Rust 侧信息；生成端回退为重导出。
    pub const fn none() -> Self {
        Self {
            #[cfg(feature = "rust")]
            derives: &[],
            #[cfg(feature = "rust")]
            attributes: &[],
            #[cfg(feature = "rust")]
            definition: None,
        }
    }

    /// 与 [`RustTypeMetadata::none`] 等价，供链式构造的起点使用。
    pub const fn new() -> Self {
        Self::none()
    }

    /// 设置派生项名称；未开启 `rust` 特性时忽略。
    pub const fn with_derives(self, derives: &'static [&'static str]) -> Self {
        #[cfg(feature = "rust")]
        {
            let mut this = self;
            this.derives = derives;
            this
        }
        #[cfg(not(feature = "rust"))]
        {
            let _ = derives;
            self
        }
    }

    /// 设置属性文本；未开启 `rust` 特性时忽略。
    pub const fn with_attributes(self, attributes: &'static [&'static str]) -> Self {
        #[cfg(feature = "rust")]
        {
            let mut this = self;
            this.attributes = attributes;
            this
        }
        #[cfg(not(feature = "rust"))]
        {
            let _ = attributes;
            self
        }
    }

    /// 设置定义快照；未开启 `rust` 特性时忽略。
    pub const fn with_definition(self, definition: fn() -> &'static str) -> Self {
        #[cfg(feature = "rust")]
        {
            let mut this = self;
            this.definition = Some(definition);
            this
        }
        #[cfg(not(feature = "rust"))]
        {
            let _ = definition;
            self
        }
    }

    /// 定义快照文本；无快照时返回 `None`。
    pub fn definition_source(self) -> Option<&'static str> {
        #[cfg(feature = "rust")]
        {
            self.definition.map(|definition| definition())
        }
        #[cfg(not(feature = "rust"))]
        {
            None
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

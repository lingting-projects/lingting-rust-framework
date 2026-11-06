# framework-proc-core

为属性宏提供运行时支撑与代码生成：元数据结构、基于 `inventory` 的静态注册表、
TypeScript 声明导出，以及 Rust crate 生成。

本 crate 不导出任何宏，宏位于 [framework-proc-auto](../framework-proc-auto/README.md)、
[framework-proc-web](../framework-proc-web/README.md)。

## feature

| feature | 作用 |
|---------|------|
| `collect`（默认关闭） | 启用基于 `inventory` 的静态注册表，提供 `type_metadata_iter`、`enum_metadata_iter`、`api_metadata_iter` |
| `ts`（默认关闭） | 编译 TypeScript 代码生成部分：`typescript` 模块及 `TypescriptTypeBuilder`、`TypescriptEnumBuilder`、`TypescriptApiBuilder`、`PackageBuilder` 等导出 |
| `rust`（默认关闭） | 编译 Rust 代码生成部分：`rust` 模块及 `RustTypeBuilder`、`RustEnumBuilder`、`RustApiBuilder`、`CrateBuilder` 等导出 |

未开启 `collect` 时，`push_*_metadata!` 宏内部的 `inventory` 路径不可用，因此注册宏与迭代函数都只在
开启该 feature 后生效。`framework-proc-auto` 通过自身的 `collect` 特性传递开启；`framework-proc-web` 不依赖
本 crate，其生成的注册代码经 `framework-web` 的 `#[doc(hidden)]` 重导出路径引用，由 `framework-web` 的
`collect` 特性一并开启本 crate 的 `collect`。

`collect`、`ts`、`rust` 三者互不依赖：`collect` 是数据来源，`ts` / `rust` 只决定哪个代码生成器参与编译。
元数据（`TypeMetadata`、`EnumMetadata`、`ApiMetadata` 及对应的迭代函数）不受 `ts` / `rust` 控制，始终可用。

只开启 `rust` 而不开启 `ts` 时，`TypeMetadata` 没有 `ts` 字段，但 `TsTypeMetadata` 仍是可构造的公开空结构体，
因此宏生成的注册代码在任意特性组合下都成立。

## 元数据

宏生成的代码通过注册宏提交静态信息，且**只调用无条件构造器**，不出现任何 `#[cfg]` 与结构体字面量：

```rust
push_type_metadata! {
    TypeMetadata::new(TypeKind::Struct, __proc_auto_type_name_User)
        .with_ts(TsTypeMetadata::register(__proc_auto_register_User))
        .with_rust(
            RustTypeMetadata::new()
                .with_derives(&["std::clone::Clone", "serde::Serialize"])
                .with_attributes(&["item:#[serde(rename_all=\"camelCase\")]"])
                .with_definition(__proc_auto_definition_User),
        )
}
```

`with_ts` / `with_rust` 在对应特性关闭时忽略参数并原样返回自身。

`TypeMetadata` 描述自动注册的类型：

| 字段 | 含义 |
|------|------|
| `kind` | `TypeKind::Struct` 或 `TypeKind::Enum` |
| `type_name` | 返回完整类型名的函数 |

`TsTypeMetadata` 描述 TypeScript 代码生成所需的信息：

| 字段 | 含义 |
|------|------|
| `register` | 可选的 specta 注册函数，`specta` 关闭时为 `None` |

`RustTypeMetadata` 描述 Rust 代码生成所需的信息：

| 字段 | 含义 |
|------|------|
| `derives` | 规范化后的派生项名称，用于生成端的依赖推断与审计 |
| `attributes` | 条目与字段上的属性文本，形如 `item:#[serde(rename_all="camelCase")]`、`field:id:#[serde_as(...)]` |
| `definition` | 清洗后的类型定义快照函数；`None` 时生成端回退为重导出 |

`EnumMetadata` 描述枚举的运行时导出：`type_name`、字段名列表 `fields`，以及生成所有枚举值的 `values`。
`EnumValue { value, fields }` 中的 `value` 是枚举的字符串值，`fields` 为字段名到 JSON 值的映射。
`enum_field_value` 用于把枚举字段值序列化为 JSON。其内容对 `ts` 与 `rust` 都是目标无关的。

`ApiMetadata` 描述一个接口：`name`、`namespace`、`method`、`path`、`parameters`、`return_type`。
参数位置由 `ApiParameterKind::Body` / `Query` 区分，返回值由 `ApiReturnKind::Void` / `Blob` / `Type` 区分。

`ApiParameterMetadata` 与 `ApiReturnType` 按目标语言分别保存类型名：

| 类型 | 构造入口 | 读取方式 |
|------|----------|----------|
| `ApiParameterMetadata` | `new(name, kind, ts_type_name, rust_type_name)` | `ts_type_name()`、`rust_type_name()`（缺失时返回 `"unknown"`） |
| `ApiReturnType` | `void()`、`blob()`、`ty(ts_type_name, rust_type_name)` | `ts_type_name()`、`rust_type_name()`（返回 `Option`） |

`ApiParameterMetadata::new` 的四个参数与 `ApiReturnType::ty` 的两个参数始终存在，对应特性关闭时被忽略。

## TypeScript 构建器

TypeScript 构建器实现 `TypescriptBuilder`，`build()` 返回 `TypescriptBuildResult { js, dts }`，
错误类型为 `TypescriptResult<T>`。以下构建器仅在开启 `ts` 特性时存在。

### TypescriptTypeBuilder

使用 specta 导出类型声明，`js` 为空字符串。`type_import_from` 目前不会改变输出（字段以下划线开头，暂未使用）。

```rust
use framework_proc_core::{TypescriptBuilder, TypescriptTypeBuilder, type_metadata_iter};

let result = TypescriptTypeBuilder::new()
    .types(type_metadata_iter())
    .type_import_from(".")
    .build()?;
# Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
```

### TypescriptEnumBuilder

为每个枚举导出 `<Name>All` 数组与 `<Name>Map` 映射。`Map` 的键为枚举值，值至少包含 `value`，
并附带枚举字段；无字段时类型为 `{ value: Name }`。字段的 TypeScript 类型由所有枚举值的实际 JSON 类型推断。

```rust
use framework_proc_core::{TypescriptBuilder, TypescriptEnumBuilder, enum_metadata_iter};

let result = TypescriptEnumBuilder::new()
    .enums(enum_metadata_iter())
    .type_import_from(".")
    .build()?;
# Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
```

`d.ts` 会先输出从 `type_import_from` 导入枚举类型的 `import type { ... }` 语句。枚举名重复时返回错误。

### TypescriptApiBuilder

导出 `ApiDefinitions` 常量及其类型工具 `ApiDefinition`、`ApiName`、`ApiArgs`、`ApiResult`、`ApiArg`，
并可选导出抽象类。

```rust
use framework_proc_core::{TypescriptApiBuilder, TypescriptBuilder, api_metadata_iter};

let result = TypescriptApiBuilder::new()
    .apis(api_metadata_iter())
    .class_name("ApiClient")
    .with_class(true)          // 默认 true；false 时只导出 ApiDefinitions
    .type_import_from(".")
    .build()?;
# Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
```

生成的抽象类包含 `protected abstract call<T>(method, path, body?, query?): Promise<T>`，
每个接口对应一个方法，`Body` 参数与 `Query` 参数分别聚合后传给 `call`。

构建时会校验元数据：方法名（经 camelCase 转换后）重复、或 `method + path` 重复都会返回错误，
错误信息中包含 `namespace::name` 便于定位。导出抽象类但未设置 `class_name`、或 `class_name` 不是合法
TypeScript 标识符时同样报错。

### PackageBuilder

把多个构建器结果写入 npm ESM 包目录：

```rust
use framework_proc_core::{PackageBuilder, TypescriptEnumBuilder, TypescriptTypeBuilder};

let mut package = PackageBuilder::new("my-api", "1.0.0", "generated");
package.push("types", TypescriptTypeBuilder::new());
package.push("enums", TypescriptEnumBuilder::new());
package.write()?;
# Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
```

输出结构：`generated/package.json` 与 `generated/dist/` 下的 `<name>.js`、`<name>.d.ts`、
`index.js`、`index.d.ts`。`package.json` 中 `type` 为 `module`，`exports` 包含根入口与每个模块入口。
模块名不能为空、不能是 `index`、不能包含路径分隔符，也不能重复。

## Rust 构建器

Rust 构建器仅在开启 `rust` 特性时存在，实现 `RustBuilder`：

```rust
pub trait RustBuilder {
    fn build(&self) -> RustResult<RustBuildResult>;
}

pub struct RustBuildResult {
    pub code: String,                 // 模块文件内容（不含 mod 声明）
    pub dependencies: Vec<RustDependency>, // 该模块需要的 Cargo 依赖
}
```

`RustResult<T>` 的错误类型是 `Box<dyn std::error::Error + Send + Sync>`。`RustDependency::new(name, version)`
可通过 `.feature(...)` 追加 feature。构建器只负责生成字符串与上报依赖，写盘由 `CrateBuilder` 完成。

### RustTypeBuilder

为每个已注册类型生成真实定义或重导出：

```rust
use framework_proc_core::{RustBuilder, RustTypeBuilder, type_metadata_iter};

let result = RustTypeBuilder::new()
    .types(type_metadata_iter())
    .type_import_from("crate::dto::*")
    .build()?;
# Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
```

- 元数据带定义快照（`rust.definition` 为 `Some`）时原样输出快照，例如
  `pub struct User { pub id : String , pub name : String , }`；快照由
  [framework-proc-auto](../framework-proc-auto/README.md) 生成，间距由 `stringify!` 决定，未做格式化。
- 没有快照时回退为 `pub use <完整类型名>;`。
- 类型按完整类型名排序输出，同名类型（最后一段标识符相同）视为重复并报错。
- `type_import_from` 可多次调用，每次生成一条 `use <source>;`。

依赖推断（`rust.derives` / `rust.attributes` 的用途）：

| 条件 | 上报依赖 |
|------|----------|
| `derives` 含 `Serialize` 或 `Deserialize`（按路径最后一段比较） | `serde = "1"`，`features = ["derive"]` |
| `attributes` 中出现 `serde_as` 或 `serde_with::` | `serde_with = "3"` |
| 以上都不满足 | 不推断依赖 |

### RustEnumBuilder

为每个枚举生成运行时信息结构体与常量表，并在设置 `type_import_from` 时追加强类型辅助函数：

```rust
use framework_proc_core::{RustBuilder, RustEnumBuilder, enum_metadata_iter};

let result = RustEnumBuilder::new()
    .enums(enum_metadata_iter())
    .type_import_from("crate::dto")
    .build()?;
# Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
```

输出示例：

```rust
pub struct Status {
    pub value: &'static str,
    pub label: &'static str,
}

pub const STATUS_ALL: &[&str] = &["Active", "Disabled"];

pub const STATUS_MAP: &[(&str, Status)] = &[
    ("Active", Status { value: "Active", label: "启用" }),
    ("Disabled", Status { value: "Disabled", label: "停用" }),
];

pub fn status_from_value(value: &str) -> Option<crate::dto::Status> { /* ... */ }
pub fn status_all() -> Vec<crate::dto::Status> { /* ... */ }
pub fn status_map() -> Vec<(crate::dto::Status, &'static Status)> { /* ... */ }
pub fn status_info(value: &str) -> Option<&'static Status> { /* ... */ }
```

- 运行时信息结构体保留枚举原名（`Status` → `Status`），常量名转换为 `SCREAMING_SNAKE_CASE`，
  辅助函数名转换为 `snake_case`；同名枚举报错。
- 常量表以枚举字符串值为键：`EnumMetadata` 只提供字符串值，无法构造 `Foo::A` 形式的变体，
  因此强类型转换由 `type_import_from` 下的辅助函数按值反序列化完成。
- 强类型枚举一律用 `type_import_from` + 枚举名拼出的全路径限定（如 `crate::dto::Status`），
  不生成 `use` 导入，避免与同名的运行时信息结构体冲突。设置 `type_import_from` 时上报 `serde_json = "1"`。
- 字段类型由所有枚举值中该字段的 JSON 形态推断：

| 所有取值中该字段的 JSON 形态 | Rust 类型 |
|------------------------------|-----------|
| 全部字符串 | `&'static str` |
| 全部布尔 | `bool` |
| 全部整数 | `i64` |
| 全部浮点，或整数与浮点混合 | `f64` |
| 其他（数组、对象）或类型不一致 | `&'static str`（JSON 文本） |
| 存在 `null` | 以上结果外包 `Option<...>`，`null` 输出为 `None` |

### RustApiBuilder

生成接口常量表与客户端 trait：

```rust
use framework_proc_core::{RustApiBuilder, RustBuilder, api_metadata_iter};

let result = RustApiBuilder::new()
    .apis(api_metadata_iter())
    .trait_name("ApiClient")
    .with_trait(true)              // 默认 true；false 时只导出接口定义
    .type_import_from("crate::types")
    .build()?;
# Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
```

输出内容分三部分：

1. `ApiArgumentKind`、`ApiArgument`、`ApiDefinition` 与每个接口一个 `ApiDefinition` 常量（常量名
   `SCREAMING_SNAKE_CASE`），最后汇总为 `API_DEFINITIONS`。
2. `ApiCallError<E>`：`Transport(E)` 表示 `send` 失败，`Format(serde_json::Error)` 表示 JSON 编解码失败。
3. `ApiClient` trait：必须实现的 `send(&self, definition, body, query)`，以及每个接口一个默认方法。

```rust
pub trait ApiClient {
    type Error: std::error::Error + Send + Sync + 'static;

    fn send(
        &self,
        definition: &ApiDefinition,
        body: Option<Vec<u8>>,
        query: Option<Vec<u8>>,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, Self::Error>> + Send;

    fn find_user(
        &self,
        id: String,
    ) -> impl std::future::Future<Output = Result<User, ApiCallError<Self::Error>>> + Send
    where
        Self: Sync,
    { /* ... */ }
}
```

- 参数与返回值类型全部取自 `RustApiTypeMetadata`；`Void` 返回 `()` 并忽略响应体，`Blob` 返回 `Vec<u8>`，
  `Type` 缺类型名时报错。
- 默认方法返回的 future 声明为 `Send`，因此要求 `Self: Sync`；默认方法内部用 `async move` 聚合参数。
- 参数聚合与 TypeScript 的 `request_value` 一致：0 个为 `None`，1 个为 `serde_json::to_vec(&arg)`，
  多个为 `serde_json::to_vec(&serde_json::json!({ "a": a, "b": b }))`。
- 参数名与 `send` / `new` / `call` / `default` / `from` / `into` 冲突、方法名重复、`method + path` 重复、
  方法名不是合法 Rust 标识符、导出 trait 但未设置合法 `trait_name`，都返回错误，信息含 `namespace::name`。
- 上报依赖 `serde = "1"` 与 `serde_json = "1"`。

### CrateBuilder

把多个构建器结果写入一个 Rust crate 目录：

```rust
use framework_proc_core::{
    CrateBuilder, RustApiBuilder, RustDependency, RustEnumBuilder, RustTypeBuilder,
};

let mut crate_builder = CrateBuilder::new("my-sdk", "1.0.0", "generated");
crate_builder.dependency(RustDependency::new("chrono", "0.4"));
crate_builder.push("types", RustTypeBuilder::new());
crate_builder.push("enums", RustEnumBuilder::new());
crate_builder.push("api", RustApiBuilder::new().trait_name("ApiClient"));
crate_builder.write()?;
# Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
```

输出结构：

```text
<output_dir>/
  Cargo.toml
  src/
    lib.rs        // pub mod a; …  后接 pub use a::*; …
    api.rs
    enums.rs
    types.rs
```

- `edition` 默认 `2024`，可用 `edition(..)` 覆盖；`rust_version(..)` 可选。
- 校验：crate 名匹配 `^[a-z][a-z0-9_-]*$`、版本非空、模块名为合法非关键字标识符且不是 `lib` / `main` 且不重复。
- 依赖按名合并，`features` 取并集，同名依赖版本不一致时报错。
- `write` 原样输出，不调用外部格式化命令。生成代码由 `stringify!` 快照拼接，缩进与换行不规整，
  建议对输出目录执行 `rustfmt <output_dir>/src/*.rs`。
- 多个模块 `pub use *` 可能产生同名符号冲突（与 TypeScript `index.d.ts` 的 `export *` 同源），
  由使用者保证不重名，构建期不检测。

### 生成结果的已知限制

- 快照中字段类型若引用其他类型，需要使用者用 `type_import_from` 覆盖其来源，否则生成 crate 编译失败；
  生成端不做静态检查，也不会校验路径是否存在。
- 快照引用的第三方 crate（如 `chrono`、`url`、`blake3`）无法推断，需要 `CrateBuilder::dependency(...)` 补充。
- 快照保留 `#[serde_as]` / `#[serde_as(as = ...)]` / `#[serde(with = ...)]` 以保证与服务端 wire 一致，
  因此生成 crate 可能需要 `serde_with` 依赖；`RustTypeBuilder` 会依据 `attributes` 推断该依赖。
- 快照丢弃 `specta::Type`、`strum_macros::*` 等使用方私有宏派生，只保留标准库派生与 serde 派生。

## utils

`utils` 模块始终编译，供本 crate 与其他宏 crate 复用：

| 函数 | 作用 |
|------|------|
| `camel_case` | `snake_case` / `kebab-case` → camelCase |
| `snake_case` | 驼峰、下划线、连字符或空格分隔 → `snake_case` |
| `shouty_snake_case` | 同上 → `SCREAMING_SNAKE_CASE` |
| `pascal_case` | 同上 → PascalCase |

```rust
use framework_proc_core::utils::{camel_case, pascal_case, shouty_snake_case, snake_case};

assert_eq!(camel_case("find_user"), "findUser");
assert_eq!(snake_case("FindUser"), "find_user");
assert_eq!(shouty_snake_case("FindUser"), "FIND_USER");
assert_eq!(pascal_case("find_user"), "FindUser");
```

## 说明

- 本 crate 重导出 `serde_json`，宏生成的代码通过 `::framework_proc_core::serde_json` 引用，避免使用方额外声明依赖。
- `__private` 模块仅在开启 `collect` 时存在，仅供注册宏内部使用。
- `specta` 是必选依赖（`TsTypeMetadata::register` 的签名需要它），不受 `ts` / `rust` 特性影响。

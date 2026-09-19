# framework-proc-core

为属性宏提供运行时支撑与 TypeScript 代码生成：元数据结构、基于 `inventory` 的静态注册表、
类型声明导出、枚举运行时导出、API 定义导出，以及打包为 npm ESM 包的 `PackageBuilder`。

本 crate 不导出任何宏，宏位于 [framework-proc-auto](../framework-proc-auto/README.md)、
[framework-proc-ts](../framework-proc-ts/README.md)、[framework-proc-web](../framework-proc-web/README.md)。

## 安装

```toml
[dependencies]
framework-proc-core = { path = "../framework-proc-core" }
```

## feature

| feature | 作用 |
|---------|------|
| `collect`（默认关闭） | 启用基于 `inventory` 的静态注册表，提供 `type_metadata_iter`、`enum_metadata_iter`、`api_metadata_iter` |

未开启 `collect` 时，`push_*_metadata!` 宏内部的 `inventory` 路径不可用，因此注册宏与迭代函数都只在
开启该 feature 后生效；宏 crate 通过自身的 `collect` feature 传递开启。

## 元数据

宏生成的代码通过注册宏提交静态信息：

```rust
framework_proc_core::push_type_metadata!(framework_proc_core::TypeMetadata { .. });
framework_proc_core::push_enum_metadata!(framework_proc_core::EnumMetadata { .. });
framework_proc_core::push_api_metadata!(framework_proc_core::ApiMetadata { .. });
```

`TypeMetadata` 描述自动注册的类型：

| 字段 | 含义 |
|------|------|
| `kind` | `TypeKind::Struct` 或 `TypeKind::Enum` |
| `type_name` | 返回完整类型名的函数 |
| `derives` | 规范化后的派生项名称 |
| `attributes` | 条目与字段上的属性文本 |
| `register` | 可选的 specta 注册函数，`specta` 关闭时为 `None` |

`EnumMetadata` 描述枚举的运行时导出：`type_name`、字段名列表 `fields`，以及生成所有枚举值的 `values`。
`EnumValue { value, fields }` 中的 `value` 是枚举的字符串值，`fields` 为字段名到 JSON 值的映射。
`enum_field_value` 用于把枚举字段值序列化为 JSON。

`ApiMetadata` 描述一个接口：`name`、`namespace`、`method`、`path`、`parameters`、`return_type`。
参数位置由 `ApiParameterKind::Body` / `Query` 区分，返回值由 `ApiReturnType::Void` / `Blob` / `Type(name)` 区分。

## 构建器

所有构建器实现 `TypescriptBuilder`，`build()` 返回 `TypescriptBuildResult { js, dts }`，
错误类型为 `TypescriptResult<T>`。

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

## camel_case

`camel_case` 供本 crate 与其他宏 crate 复用，用于把 `snake_case` 或 `kebab-case` 转为 camelCase：

```rust
use framework_proc_core::camel_case;

assert_eq!(camel_case("find_user"), "findUser");
```

## 说明

- 本 crate 重导出 `serde_json`，宏生成的代码通过 `::framework_proc_core::serde_json` 引用，避免使用方额外声明依赖。
- `__private` 模块仅在开启 `collect` 时存在，仅供注册宏内部使用。

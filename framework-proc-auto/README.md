# framework-proc-auto

提供 `auto_type`、`auto_enum`、`auto_enum_impl`、`auto_enum_field` 四个属性宏，
用一行注解补齐 serde、specta、strum 派生与命名转换，并可选地注册类型元数据。

## 安装

```toml
[dependencies]
framework-proc-auto = { path = "../framework-proc-auto" }
```

宏生成的代码会引用以下 crate，使用方需要在自身依赖中声明：

| 生成代码引用 | 何时需要 |
|--------------|----------|
| `serde` | 未关闭 `serde` 参数 |
| `strum`、`strum_macros` | 未关闭 `strum` 参数（`auto_enum`） |
| `specta` | 未关闭 `specta` 参数 |
| `serde_with` | 结构体字段包含需 `DisplayFromStr` 转换的类型 |
| `specta_typescript` | 字段类型为 `serde_json::Value` |

## feature

| feature | 作用 |
|---------|------|
| `collect`（默认关闭） | 生成类型与枚举元数据注册代码，同时开启 `framework-proc-core/collect` |

未开启 `collect` 时，`auto_type` 与 `auto_enum` 只做属性加工，不生成任何注册代码；
`auto_enum_impl` 会移除 `#[auto_enum_field]` 标记但不再生成导出函数。

## auto_type

用于具名字段结构体，仅支持类型泛型（不支持生命周期与常量泛型）。

```rust
use framework_proc_auto::auto_type;

#[auto_type]
pub struct PaginationParams {
    #[specta(type = i32)]
    pub current: i64,
    #[specta(type = i32)]
    pub size: i64,
    pub sorts: Vec<PaginationSort>,
}

#[auto_type]
pub struct PaginationSort {
    pub field: String,
    pub desc: bool,
}
```

参数（均为 `name = true/false` 形式）：

| 参数 | 默认值 | 作用 |
|------|--------|------|
| `default` | `true` | 派生 `Default` |
| `clone` | `false` | 派生 `Clone` |
| `copy` | `false` | 派生 `Copy`，并隐含 `Clone` |
| `eq` | `true` | 派生 `PartialEq` 与 `Eq` |
| `serde` | `true` | 派生 `Serialize`、`Deserialize` |
| `specta` | `true` | 派生 `specta::Type` |

`Debug` 始终派生。已有的 `#[derive(...)]` 会被保留，缺失项按需追加；判定重复时只比较路径最后一段，
因此手写 `serde::Serialize` 与自动追加的 `Serialize` 不会冲突。

`serde` 开启时还会追加：

- 条目级 `#[serde(rename_all = "camelCase")]`，已存在 `rename_all` 时不覆盖。
- `Option<T>` 字段追加 `#[serde(default)]`。
- 需要适配器的字段追加 `#[serde_as(as = "...")]`，并在条目上插入 `#[serde_with::serde_as]`（插入到属性首位）。

## 类型转换

`auto_type` 与 `auto_enum_impl` 共用同一套类型识别，按字段类型自动补齐转换：

| Rust 类型 | serde 处理 | specta 处理 |
|-----------|------------|-------------|
| `u64`、`i64`、`u128`、`i128`、`Uuid`、`Decimal`、`DateTime` | `#[serde_as(as = "DisplayFromStr")]` | `#[specta(type = String)]` |
| `serde_json::Value` | 不变 | `#[specta(type = specta_typescript::Unknown)]` |
| `Option<T>` | 按 `T` 处理并包裹 `Option` | 同左 |
| `Vec<T>` | 按 `T` 处理并包裹 `Vec` | 同左 |
| `HashMap<K, V>`、`Map<K, V>` | 仅在 `K` 或 `V` 需要时包裹 | 同左 |
| `BTreeMap<K, V>` | 同上 | 同上 |

已存在同名属性（`serde_as` 的 `as`、`specta` 的 `type`）时不会覆盖。

## auto_enum

用于无泛型枚举，同时对齐 strum 与 serde 的命名：

```rust
use framework_proc_auto::auto_enum;

#[auto_enum]
pub enum RCodeKind {
    Success,
    Parameter,
    Unauthorized,
    Forbidden,
    Internal,
}
```

参数：

| 参数 | 默认值 | 作用 |
|------|--------|------|
| `clone` | `true` | 派生 `Clone` |
| `copy` | `true` | 派生 `Copy`，并隐含 `Clone` |
| `eq` | `true` | 派生 `PartialEq` 与 `Eq` |
| `strum` | `true` | 派生 `EnumIter`、`EnumString`、`Display` |
| `serde` | `true` | 派生 `Serialize`、`Deserialize` |
| `specta` | `true` | 派生 `specta::Type` |

命名转换固定为 `SCREAMING_SNAKE_CASE`：

- `strum` 开启时追加 `#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]` 与 `#[strum(ascii_case_insensitive)]`，
  因此 `Display`、`to_string()` 与 `FromStr` 都使用该命名，且解析不区分大小写。
- `serde` 开启时追加 `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]`。

单单词变体（如 `Success`）两种命名的结果相同，均为 `SUCCESS`；多单词变体（如 `FooBar`）为 `FOO_BAR`。
`Debug` 始终派生。

## auto_enum_impl

用于枚举的固有 `impl`，为枚举导出运行时字段。不支持 trait impl、泛型 impl 与宏参数。

```rust
use framework_proc_auto::{auto_enum_impl, auto_enum_field};

#[auto_enum_impl]
impl RCodeKind {
    #[auto_enum_field]
    pub fn code(&self) -> u32 {
        match self {
            RCodeKind::Success => 200,
            RCodeKind::Parameter => 400,
            RCodeKind::Unauthorized => 401,
            RCodeKind::Forbidden => 403,
            RCodeKind::Internal => 500,
        }
    }
}
```

字段收集规则：

- 带 `#[auto_enum_field]` 的方法会被收集；未指定 `field = "..."` 时，字段名取方法名的 camelCase 形式，
  例如 `find_user` 对应 `findUser`。
- 名为 `label` 的方法在未带标记时也会被收集，字段名固定为 `label`。
- `#[auto_enum_field(field = "code")]` 可显式指定字段名，参数必须是非空字符串，且每个方法只能指定一次。
- 字段名不能重复。

方法约束：必须接收 `self` 且不能有其他参数，不能是 `async` 或泛型方法，必须声明返回类型。

返回值按类型转换为 JSON 值：

- `u64`、`i64`、`u128`、`i128`、`Uuid`、`Decimal`、`DateTime`：转为 JSON 字符串。
- `Option<T>`：`Some` 按 `T` 处理，`None` 转为 JSON `null`。
- `Vec<T>`：转为 JSON 数组；`Option<Vec<T>>` 组合两者。
- 其他类型（含 `String`）：调用 `framework_proc_core::enum_field_value`，即按 `Serialize` 序列化，失败时返回错误。

开启 `collect` 后，导出的元数据包含枚举值列表（由 `strum::IntoEnumIterator` 遍历，值为 `to_string()` 结果）
与字段名列表，供 `framework-proc-core` 的 `TypescriptEnumBuilder` 生成 TypeScript 运行时导出。

## 与其他 crate 的关系

- 类型与枚举元数据的结构、注册表与 TypeScript 导出位于 [framework-proc-core](../framework-proc-core/README.md)。
- `auto_type` / `auto_enum` 生成的 `specta::Type` 派生需要 specta 侧开启 serde 集成，
  才能让 TypeScript 类型与 serde 的命名转换保持一致。

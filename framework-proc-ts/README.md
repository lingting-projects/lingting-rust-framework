# framework-proc-ts

提供 `ts_api` 属性宏，从 Rust 函数签名推导 TypeScript 接口信息并注册为 `ApiMetadata`。

宏本身不生成 TypeScript 代码，只做元数据收集；生成代码位于
[framework-proc-core](../framework-proc-core/README.md) 的 `TypescriptApiBuilder`。
[framework-proc-web](../framework-proc-web/README.md) 的 `web_api` 宏内部会自动叠加 `ts_api`。

## 安装

```toml
[dependencies]
framework-proc-ts = { path = "../framework-proc-ts" }
```

## feature

| feature | 作用 |
|---------|------|
| `collect`（默认关闭） | 生成 `ApiMetadata` 注册代码，同时开启 `framework-proc-core/collect` |

未开启 `collect` 时宏只校验签名，不产生任何代码。

## 用法

```rust
use framework_proc_ts::ts_api;

#[ts_api(method = "GET", path = "user/find")]
pub async fn find_user(params: Query<FindUserParams>) -> Result<UserVO, anyhow::Error> {
    // ...
}
```

参数必须使用 `name = value` 形式，且 `method` 与 `path` 均为必填字符串。

## 参数规则

每个函数参数必须能识别出传输位置：

| 参数类型 | 位置 | TypeScript 类型来源 |
|----------|------|---------------------|
| `Json<T>` | `Body` | `T` |
| `Query<T>` | `Query` | `T` |
| `PaginationParams` | `Body` | `PaginationParams` |

其他类型会报错。参数不能是 `self`，也不能是 `WebContext`（需在函数内调用 `use_web()`）。
参数名取自模式中的唯一绑定名，`Json(x)`、`Query(x)` 这类单元素元组结构体模式会被解包，
因此 `Json(params)` 的参数名为 `params`。

## 返回值规则

先逐层剥掉 `Result<T>` 与 `R<T>` 包装，再按下表判定：

| 剥壳后的类型 | `ApiReturnType` |
|--------------|-----------------|
| 省略返回类型或 `()` | `Void` |
| `WebResponse` | `Blob` |
| 其他类型 | `Type(name)` |

## 类型名转换

`Type(name)` 与参数类型名由 Rust 类型名转换为 TypeScript 文本：

| Rust 类型 | TypeScript 文本 |
|-----------|-----------------|
| `String`、`str`、`char` | `string` |
| `bool` | `boolean` |
| `i8`、`i16`、`i32`、`u8`、`u16`、`u32`、`isize`、`usize`、`f32`、`f64` | `number` |
| `i64`、`u64`、`i128`、`u128` | `string` |
| `Option<T>` | `T \| null` |
| `Vec<T>` | `T[]` |
| `HashMap<K, V>`、`BTreeMap<K, V>` | `Record<K, V>` |
| 无泛型参数的类型 | 最后一段标识符，如 `UserVO` |
| 其他泛型类型 | `Name<A, B>` |

泛型参数递归转换，因此 `Option<Vec<UserVO>>` 得到 `UserVO[] | null`。
不支持的类型（如元组、函数指针、带括号的泛型参数）会报错。

`i64`、`u64`、`i128`、`u128` 映射为 `string`，与 `auto_type` 对同名字段的 `DisplayFromStr` 处理保持一致。
因此裸 `i64` 返回值会被标注为 `string`，而 serde 实际输出的是数字；返回值中的 id 请用 `auto_type`
结构体包装，避免类型标注与运行时不一致。

## 注册结果

开启 `collect` 后，宏注册的 `ApiMetadata` 包含：

- `name`：函数名。
- `namespace`：`module_path!()`。
- `method`、`path`：宏参数。
- `parameters`：参数名、TypeScript 类型名与位置。
- `return_type`：返回值描述。

`TypescriptApiBuilder` 会基于这些信息生成 `ApiDefinitions`、抽象类与类型导入，
并对方法名重复、`method + path` 重复做校验。

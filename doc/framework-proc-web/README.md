# framework-proc-web

提供 `web_api` 及 HTTP 方法简化宏，把 `async fn` 展开为可调用的 `WebRoute` 构造函数，
并在开启 `collect` 特性时直接生成 [framework-proc-core](../framework-proc-core/README.md) 的 `ApiMetadata` 注册代码。

宏只生成路由与参数提取代码，运行时的请求上下文、错误处理、响应构造位于
[framework-web](../framework-web/README.md)。

## 使用

生成的代码引用 `framework_web`；开启 `collect` 特性时还会引用
`framework_web::framework_proc_core`，该路径由 [framework-web](../framework-web/README.md)
以 `#[doc(hidden)]` 重导出，使用方无需再声明 `framework_proc_core` 依赖。

## feature

| feature | 作用 |
|---------|------|
| `collect`（默认关闭） | 额外生成 `push_web_api!` 路由注册与 `push_api_metadata!` 接口元数据注册代码 |

`framework-web` 的 `collect` 会传递开启本 crate 的 `collect`，并同时开启
`framework-proc-core/collect` 让元数据注册真正生效；未开启时路由构造函数与接口函数照常可用，
但不会自动注册，需要手动挂载。

## 宏

| 宏 | HTTP 方法 |
|----|-----------|
| `web_api` | 由 `method` 参数指定 |
| `web_api_get` | `GET` |
| `web_api_post` | `POST` |
| `web_api_put` | `PUT` |
| `web_api_patch` | `PATCH` |
| `web_api_delete` | `DELETE` |

简化宏不接受 `method` 参数，重复指定会报错。

## 参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `method` | 使用 `web_api` 时必填 | HTTP 方法标识符，如 `get`、`post`、`delete`，大小写不敏感 |
| `path` | 是 | 字符串，两端 `/` 会被裁掉，裁剪后不能为空 |
| `auth` | 否 | 授权规则表达式，默认 `framework_web::AuthRule::login()` |

```rust
use framework_proc_web::web_api_post;
use framework_web::{AuthRule, Json};

#[web_api_post(path = "user/create", auth = AuthRule::anonymous())]
pub async fn create_user(params: Json<CreateUserParams>) -> Result<UserVO, anyhow::Error> {
    // ...
}
```

## 函数要求

- 必须是 `async fn`。
- 不支持 `self` 参数。
- 不支持 `WebContext` 参数，需在函数内调用 `framework_web::use_web()`。
- 每个参数类型必须实现 `FromWeb`，参数在进入函数体前逐个 `await` 转换，
  转换失败直接返回对应的错误响应。
- `GET` 接口最多只能有一个参数（一个从查询参数提取的参数对象）。

## 返回值处理

按函数返回类型选择响应构造方式：

| 返回类型 | 生成的响应 |
|----------|------------|
| 省略或 `()` | 执行后返回 `WebResponse::empty()`（204） |
| `WebResponse` | 直接返回 |
| `R<T>` | `WebResponse::from_r` |
| `T` | `WebResponse::from_t`，包装为 `R::ok` |
| `Result<WebResponse, E>` | `WebResponse::from_result` |
| `Result<R<T>, E>` | `WebResponse::from_result_r` |
| `Result<T, E>` | `WebResponse::from_result_t` |

## 生成内容

对于函数 `create_user`，宏生成：

1. 原函数。
2. `pub(crate) fn build_create_user_route() -> framework_web::WebRoute`。

路由的 `invoke` 闭包内依次完成：读取 `use_web()` 上下文、提取各参数、
在 `catch_panic` 中执行函数体与响应构造。panic 与参数转换错误都会转为错误响应并记录日志。

开启 `collect` 时，还会生成 `framework_web::push_web_api!(build_create_user_route);`，
供 `framework-web-axum` 的 `WebRouter` 自动收集。

开启 `collect` 时还会生成 `framework_web::framework_proc_core::push_api_metadata!` 调用，
把接口签名登记到 API 元数据注册表，供生成 TypeScript / Rust 代码使用。

## 接口元数据（`collect` 特性）

开启 `collect` 后，宏按函数签名生成 `ApiMetadata` 注册代码，由
[framework-proc-core](../framework-proc-core/README.md) 的 `TypescriptApiBuilder` 与 `RustApiBuilder` 消费；
宏本身不生成代码文本，构建器代码分别由 `framework-proc-core` 的 `ts` / `rust` 特性提供。

### 参数规则

| 参数类型 | 位置 | 类型名来源 |
|----------|------|-----------|
| `Json<T>` | `Body` | `T` |
| `Query<T>` | `Query` | `T` |
| `PaginationParams` | `Body` | `PaginationParams` |

开启 `collect` 时其他参数类型会报错，因此自定义 `FromWeb` 提取器不能与 `collect` 同时使用。
参数名取自模式中的唯一绑定名，`Json(params)` 这类单元素元组结构体模式会被解包。

每个参数都按目标语言同时登记两个类型名，通过
`ApiParameterMetadata::new(name, kind, ts_type_name, rust_type_name)` 一次性写入。

### 返回值规则

先逐层剥掉 `Result<T>` 与 `R<T>` 包装，再按下表判定：

| 剥壳后的类型 | `ApiReturnType` |
|--------------|-----------------|
| 省略返回类型或 `()` | `void()` |
| `WebResponse` | `blob()` |
| `WebResponseOf<T>` | `ty(ts_name, rust_name)` |
| 其他类型 | `ty(ts_name, rust_name)` |

`ApiReturnType` 只记录类别（`ApiReturnKind::Void` / `Blob` / `Type`）与两个目标语言的类型名，
`Type` 以外不记录类型名。TypeScript 侧 `Void` 输出 `void`、`Blob` 输出 `Blob`；
Rust 侧 `Void` 输出 `()`、`Blob` 输出 `Vec<u8>`。

### 类型名转换

参数与返回值的类型名由 Rust 类型同时转换为两个目标语言的文本：

| Rust 类型 | TypeScript 文本 | Rust 文本 |
|-----------|-----------------|-----------|
| `String`、`str`、`char` | `string` | `String` |
| `bool` | `boolean` | `bool` |
| `i8`、`i16`、`i32`、`u8`、`u16`、`u32`、`isize`、`usize`、`f32`、`f64` | `number` | 原名 |
| `i64`、`u64`、`i128`、`u128` | `string` | 原名 |
| `Option<T>` | `T \| null` | `Option<T>` |
| `Vec<T>` | `T[]` | `Vec<T>` |
| `HashMap<K, V>`、`BTreeMap<K, V>` | `Record<K, V>` | `HashMap<K, V>` / `BTreeMap<K, V>` |
| 无泛型参数的类型 | 最后一段标识符，如 `UserVO` | 最后一段标识符 |
| 其他泛型类型 | `Name<A, B>` | `Name<A, B>` |

泛型参数递归转换，因此 `Option<Vec<UserVO>>` 得到 `UserVO[] | null` 与 `Option<Vec<UserVO>>`。
不支持的类型（如元组、函数指针、带括号的泛型参数）会报错。

`i64`、`u64`、`i128`、`u128` 在 TypeScript 侧映射为 `string`，与 `auto_type` 对同名字段的 `DisplayFromStr`
处理保持一致；Rust 侧保留原名，因此不产生有损转换。TypeScript 侧裸 `i64` 返回值会被标注为 `string`，
而 serde 实际输出的是数字；返回值中的 id 请用 `auto_type` 结构体包装，避免类型标注与运行时不一致。

### 注册结果

开启 `collect` 后注册的 `ApiMetadata` 包含：

- `name`：函数名。
- `namespace`：`module_path!()`。
- `method`、`path`：宏参数。
- `parameters`：参数名、位置，以及 TypeScript 与 Rust 两个类型名。
- `return_type`：返回值类别与两个目标语言的类型名。

`TypescriptApiBuilder` 会基于 TypeScript 类型名生成 `ApiDefinitions`、抽象类与类型导入，
`RustApiBuilder` 会基于 Rust 类型名生成 `ApiDefinition` 常量与 `ApiClient` trait，
二者都对方法名重复、`method + path` 重复做校验。

## 说明

- 宏生成的辅助变量使用 `__web_argument_<索引>` 命名，避免与函数参数冲突。
- 路径不做前缀拼接与参数占位符解析，`WebRouter` 按 `path` 字符串精确匹配。

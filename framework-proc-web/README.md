# framework-proc-web

提供 `web_api` 及 HTTP 方法简化宏，把 `async fn` 展开为可调用的 `WebRoute` 构造函数，
并自动叠加 [framework-proc-ts](../framework-proc-ts/README.md) 的 `ts_api` 元数据收集。

宏只生成路由与参数提取代码，运行时的请求上下文、错误处理、响应构造位于
[framework-web](../framework-web/README.md)。

## 安装

```toml
[dependencies]
framework-proc-web = { path = "../framework-proc-web" }
```

生成的代码引用 `framework_web`、`framework_proc_ts` 与 `framework_proc_core`，
使用方需要自行声明这些依赖（通常通过 `framework-web` 间接获得）。

## feature

| feature | 作用 |
|---------|------|
| `collect`（默认关闭） | 额外生成 `push_web_api!` 调用，把路由注册进 `inventory` |

`framework-web` 的 `collect` 会传递开启本 feature；未开启时路由构造函数依然生成，
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

1. 原函数，并叠加 `#[framework_proc_ts::ts_api(method = "...", path = "...")]`。
2. `pub(crate) fn build_create_user_route() -> framework_web::WebRoute`。

路由的 `invoke` 闭包内依次完成：读取 `use_web()` 上下文、提取各参数、
在 `catch_panic` 中执行函数体与响应构造。panic 与参数转换错误都会转为错误响应并记录日志。

开启 `collect` 时，还会生成 `framework_web::push_web_api!(build_create_user_route);`，
供 `framework-web-axum` 的 `WebRouter` 自动收集。

## 说明

- 宏生成的辅助变量使用 `__web_argument_<索引>` 命名，避免与函数参数冲突。
- 路径不做前缀拼接与参数占位符解析，`WebRouter` 按 `path` 字符串精确匹配。

# framework-web

Web 层运行时核心：请求与响应模型、请求上下文、错误分类与日志、参数提取 trait、授权规则与路由模型。
框架无关，HTTP 服务器实现见 [framework-web-axum](../framework-web-axum/README.md)。

## 安装

```toml
[dependencies]
framework-web = { path = "../framework-web" }
```

## feature

| feature | 作用 |
|---------|------|
| `collect`（默认关闭） | 启用基于 `inventory` 的路由收集，提供 `web_api_iter`，并传递开启 `framework-proc-web/collect` |

## 请求

`WebRequest` 为服务器适配层填充的请求快照：

| 字段 | 说明 |
|------|------|
| `method` | `WebMethod` |
| `scheme`、`authority` | 协议与主机 |
| `path` | 请求路径，匹配时忽略两端 `/` |
| `headers` | 请求头，`MultiStringValue` |
| `query` | 查询参数，`MultiStringValue` |
| `body` | 原始请求体 `Bytes` |
| `client_ip` | 客户端地址，可选 |
| `trace_id` | 链路标识：优先取 `x-trace-id` 请求头，缺失时由服务器生成雪花 ID |
| `receive_time` | 服务器收到请求的时刻，毫秒时间戳 |

`WebMethod` 包含 `Get`、`Post`、`Put`、`Patch`、`Delete`、`Options` 与 `Other(String)`。
`WebMethod::from_name` 会转大写后匹配，未识别的方法落入 `Other`。

## 上下文

`WebContext` 持有 `Arc<WebRequest>`，并缓存解析后的 JSON：

- `request()`、`request_arc()`、`query()`、`body()`：直接访问原始数据。
- `body_json()`：解析请求体 JSON 并缓存，失败返回参数错误。
- `query_json()`：把查询参数转为 JSON 对象，单值转字符串、多值转数组，并缓存。

上下文通过 Tokio task-local 传递：

```rust
use framework_web::{scope_web, use_web};

scope_web(context, async {
    let context = use_web()?;      // 在作用域内可获取
    let json = context.body_json()?;
    # Ok::<(), anyhow::Error>(())
}).await;
```

不在作用域内时 `use_web()` 返回错误。

## 参数提取

`FromWeb` 是参数提取 trait，由 [framework-proc-web](../framework-proc-web/README.md) 生成的代码调用。

| 类型 | 行为 |
|------|------|
| `Json<T>` | `GET` 请求从查询参数提取，其他方法从请求体提取 |
| `Query<T>` | 始终从查询参数提取 |
| `PaginationParams` | 从查询参数或请求体提取，并做分页校验 |

分页校验规则：`current` 小于等于 0 时取 1，`size` 小于等于 0 时取 10，超过 100 时报错，
排序字段必须由字母、数字或下划线组成且以字母或下划线开头，否则报错。

## 响应

`WebResponse` 由 `status`、`headers`、`body` 组成，`body` 为 `WebBody`：

- `WebBody::Bytes(Bytes)`：一次性返回，`String`、`Vec<u8>`、`Bytes` 均可 `From` 转换。
- `WebBody::Stream(BoxStream<'static, Result<Bytes, std::io::Error>>)`：流式返回，用于 SSE 等场景，
  通过 `is_stream()` 判断。

构造方法：

| 方法 | 说明 |
|------|------|
| `empty()` | 204 空响应 |
| `from_t(value, request)` | 包装为 `R::ok(value)`，JSON 响应 |
| `from_r(r, request)` | 序列化 `R<T>` |
| `from_result_t` / `from_result_r` | 从 `Result` 转换，错误转为错误响应 |
| `from_result(response, request)` | 从 `Result<WebResponse>` 转换 |
| `from_error(error, request)` | 错误响应，同时记录日志 |
| `from_error_request(error, trace_id, method, path)` | 无 `WebRequest` 时记录日志 |
| `stream(status, content_type, stream)` | 流式响应 |

JSON 响应会设置 `content-type: application/json; charset=utf-8` 与 `content-length`。
序列化失败会转为返回值转换错误。

## 错误

`WebError` 按 `WebErrorKind` 分类，状态码复用 [framework-core](../framework-core/README.md) 的 `RCodeKind`：

| 类型 | 状态码 | 对外消息 |
|------|--------|----------|
| `Message` | 500 | 原消息 |
| `Parameter` | 400 | 原消息 |
| `Unauthorized` | 401 | 原消息 |
| `Forbidden` | 403 | 原消息 |
| `NotFound` | 404 | 原消息 |
| `ReturnConversion` | 500 | `服务器内部错误` |
| `Internal` | 500 | `服务器内部错误` |
| `Panic` | 500 | `服务器内部错误` |

构造方法：`message`、`parameter`、`return_conversion`、`not_found`、`unauthorized`、`forbidden`、
`internal`、`panic`、`with_source`。带 `source` 的构造方法在 debug 构建下会记录源码位置。

`log` / `log_request` 输出包含分类、状态码、`trace_id`、方法、路径、源码位置、
错误链与 backtrace 的 `error!` 日志。`source_backtrace` 取 `anyhow` 的 backtrace。

`WebErrorExt` 为 `Result<T, E>` 提供快捷转换：`message()`、`parameter(msg)`、`internal(msg)`。

`WebResponse::from_error` 接受 `anyhow::Error`，内部会向下转型：`WebError` 直接使用，
`&'static str` 与 `String` 归为 `Message`，其他归为 `Internal`。

## 授权

`AuthRule` 描述接口的访问要求，序列化字段名为 camelCase：

| 字段 | 语义 |
|------|------|
| `anonymous` | `Some(true)` 时跳过全部校验 |
| `organizations` / `organizations_any` | 全部满足 / 任一满足 |
| `permissions` / `permissions_any` | 同上 |
| `roles` / `roles_any` | 同上 |
| `rules` | 子规则，需全部满足 |
| `rules_any` | 子规则，需任一满足 |

`AuthRule::login()` 为默认值（`anonymous = false`），`AuthRule::anonymous()` 允许匿名。
`check(organizations, permissions, roles)` 汇总校验；未提供某类值而规则要求该类时校验失败，
提供了空列表且规则有要求时同样失败。

授权规则的执行由服务器适配层决定，见 `framework-web-axum` 的路由包装器。

## 路由

`WebRoute` 由 `method`、`path`、`auth`、`invoke` 组成，`invoke` 为
`Arc<dyn Fn() -> BoxFuture<'static, WebResponse> + Send + Sync>`。

开启 `collect` 后：

- `WebApiBuilder(pub fn() -> WebRoute)` 作为 `inventory` 注册项。
- `push_web_api!(builder)` 宏用于提交注册。
- `web_api_iter()` 返回所有已注册路由。

`catch_panic` 把异步任务的 panic 转为 `WebError::Panic` 错误，宏生成的代码在调用函数体前会包裹它。

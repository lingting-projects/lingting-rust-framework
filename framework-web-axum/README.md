# framework-web-axum

基于 axum 的 HTTP 服务器适配层：请求分发、路由收集、CORS 处理与服务启动。
请求/响应模型与错误处理由 [framework-web](../framework-web/README.md) 提供。

## 安装

```toml
[dependencies]
framework-web-axum = { path = "../framework-web-axum" }
```

## feature

| feature | 作用 |
|---------|------|
| `collect`（默认开启） | 传递开启 `framework-web/collect`，使 `web_api` 宏生成的路由被自动收集 |

## 启动服务

```rust
use framework_web_axum::axum_builder;

let server = axum_builder("127.0.0.1", 8080).bind().await?;
server.run(None).await?;
# Ok::<(), anyhow::Error>(())
```

- `axum_builder(bind_address, bind_port)`：`bind_port` 小于等于 0 或超出 `u16` 范围时使用随机端口。
- `AxumBuilder::with_cors(f)`：设置 CORS 解析函数，默认拒绝跨域。
- `AxumBuilder::bind()`：绑定端口，返回 `AxumServer`；端口为 0 时由系统分配。
- `AxumServer::context()`：返回 `AxumContext`，其中 `port` 是实际绑定端口。
- `AxumServer::router()`：服务启动后可取到 `Arc<WebRouter>`，未启动时为 `None`。
- `AxumServer::run(wrapper)`：启动并持续运行；重复调用返回错误。

## 路由

`WebRouter::new(wrapper)` 在构造时通过 `framework_web::web_api_iter()` 收集所有已注册路由，
按 `WebMethod` 与 `path` 建索引。查找时忽略请求路径两端的 `/`，匹配不到返回 404。

`WebRouteWrapper` 用于在命中路由后包裹执行，可用于授权校验、上下文注入等：

```rust
use framework_web_axum::WebRouteWrapper;
use std::sync::Arc;

let wrapper: WebRouteWrapper = Arc::new(|route| {
    Box::pin(async move { Ok(route.invoke().await) })
});
```

未提供 wrapper 时使用默认实现，直接执行 `route.invoke()`。

`WebRouter::invoke(web_context, axum_context)` 会依次注入 `AxumContext` 与 `WebContext`
两层 task-local 作用域，再查找并执行路由。

## 分发流程

`dispatch` 作为 axum 的 fallback 处理所有请求：

1. 读取 `x-request-id` 请求头，为空时用 `next_id()` 生成雪花 ID。
2. 收集请求头（键名转小写）与查询参数（保留原大小写），读取请求体，上限 16 MiB。
3. 组装 `WebRequest`：`scheme` 缺失时取 `http`，`authority` 缺失时取 `host` 请求头，
   `client_ip` 取连接对端 IP。
4. `OPTIONS` 请求直接返回 `WebResponse::empty()`（204），不进入路由查找。
5. 其他请求在 `catch_panic` 中执行 `WebRouter::invoke`，panic 与错误都转为错误响应。
6. 应用 CORS 响应头，写入 `x-request-id`，转换为 axum `Response`。

响应体支持一次性 `Bytes` 与流式 `BoxStream`，分别映射为 `Body::from` 与 `Body::from_stream`。

分发阶段自身出错（如请求体读取失败）时，会转换为错误响应；若连响应构造都失败，
返回 500 空响应并记录日志。

## CORS

`AxumCors` 是响应头的简短映射，`None` 表示不输出该响应头：

| 字段 | 响应头 |
|------|--------|
| `origin` | `access-control-allow-origin` |
| `methods` | `access-control-allow-methods` |
| `headers` | `access-control-allow-headers` |
| `expose` | `access-control-expose-headers` |
| `credentials` | `access-control-allow-credentials` |
| `max_age` | `access-control-max-age` |

`AxumCors::fixed()` 返回不输出任何 CORS 头的静态实例，即拒绝跨域，也是默认行为。
`AxumCors::apply(&mut headers)` 把配置写入响应头。

`with_cors` 接收 `Fn(&WebContext) -> AxumCors`，可按请求动态决定 CORS 配置。

## 上下文

`AxumContext` 记录绑定地址与实际端口：

```rust
use framework_web_axum::{scope_axum, use_axum};

let context = use_axum()?;
let address = context.address();   // 形如 127.0.0.1:8080
# Ok::<(), anyhow::Error>(())
```

`scope_axum(context, future)` 建立作用域，`use_axum()` 在作用域内获取上下文，否则返回错误。

## 组装

`build_router(wrapper)` 创建 `WebRouter`；`build_router_with(router, context, cors)`
把路由组装为 axum `Router`，并挂载统一的 `fallback` 分发。

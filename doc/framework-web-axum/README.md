# framework-web-axum

基于 axum 的 HTTP 服务器适配层：请求分发、路由收集、CORS 处理、服务启动与优雅关闭。
请求/响应模型与错误处理由 [framework-web](../framework-web/README.md) 提供。

## feature

| feature | 作用 |
|---------|------|
| `collect`（默认开启） | 传递开启 `framework-web/collect`，使 `web_api` 宏生成的路由被自动收集 |
| `ts`（默认关闭） | 传递开启 `framework-web/ts`，开启 `collect` 并提供 TypeScript 代码生成构建器 |
| `rust`（默认关闭） | 传递开启 `framework-web/rust`，开启 `collect` 并提供 Rust crate 生成构建器 |

## 启动服务

```rust
use framework_web_axum::axum_builder;

let server = axum_builder("127.0.0.1", 8080).bind().await?;
server.run(None).await?;
# Ok::<(), anyhow::Error>(())
```

- `axum_builder(bind_address, bind_port)`：`bind_port` 小于等于 0 或超出 `u16` 范围时使用随机端口。
- `AxumBuilder::with_cors(f)`：设置 CORS 解析函数，默认拒绝跨域。
- `AxumBuilder::push_interceptor(interceptor)`：追加拦截器，可多次调用，执行顺序按 `order()` 升序。
- `AxumBuilder::bind()`：绑定端口，返回 `AxumServer`；端口为 0 时由系统分配。
- `AxumServer::context()`：返回 `AxumContext`，其中 `port` 是实际绑定端口。
- `AxumServer::router()`：服务启动后可取到 `Arc<WebRouter>`，未启动时为 `None`。
- `AxumServer::run(wrapper)`：启动并持续运行，收到优雅关闭请求后返回 `Ok(())`；重复调用返回错误。
- `AxumServer::shutdown()`：取优雅关闭句柄 `AxumShutdown`，`run` 期间也可取用。
- `AxumContext::status()`：当前运行状态 `AxumStatus`。
- `AxumContext::subscribe_stopping()`：等待服务进入停止流程，用于回收资源。

## 运行状态

`AxumStatus` 只能由服务内部推进，外部通过 `AxumContext::status()` 读取：

| 状态 | 含义 |
|------|------|
| `Unstarted` | 未运行：已绑定端口，尚未启动 |
| `Starting` | 启动中：正在组装路由、准备开始服务 |
| `Running` | 运行中：正常接受请求 |
| `Stopping` | 停止中：已触发优雅关闭，不再处理新请求，等待进行中的请求结束 |
| `Stopped` | 已停止：服务已退出 |

- `AxumStatus::is_running()` 仅在 `Running` 时为 `true`。
- `AxumStatus::is_stopping()` 在 `Stopping` 与 `Stopped` 时为 `true`，即"停止流程"。
- `bind()` 之后为 `Unstarted`；`run()` 依次推进 `Starting` → `Running`，收到关闭请求进入 `Stopping`，
  `axum::serve` 返回后进入 `Stopped`。
- 关闭请求早于 `run()` 时状态变为 `Stopping`，`run()` 不启动服务、置 `Stopped` 并直接返回 `Ok(())`。
- 关闭句柄全部释放不会触发关闭；`AxumContext::new` 手工创建的上下文不与任何服务共享状态，恒为 `Unstarted`。

## 优雅关闭

`AxumShutdown` 是 `Clone` 的关闭句柄，可在任意任务、路由或定时器中触发：

```rust
use framework_web_axum::axum_builder;

let server = axum_builder("127.0.0.1", 8080).bind().await?;
let shutdown = server.shutdown();

tokio::spawn({
    let shutdown = shutdown.clone();
    async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            shutdown.shutdown();
        }
    }
});

// 关闭完成后返回 Ok(())
server.run(None).await?;
# Ok::<(), anyhow::Error>(())
```

- `AxumShutdown::shutdown()`：触发优雅关闭，幂等，已处于停止流程时不做任何事。
- `AxumShutdown::is_shutdown()`：是否已进入停止流程。
- 触发关闭后停止接受新连接，等待进行中的请求结束后 `run()` 返回 `Ok(())`；运行出错时先置 `Stopped` 再返回错误。
- 停止流程中到达的新请求由分发层直接拒绝（见[分发流程](#分发流程)）。
- 外部可在收到停止通知后回收资源：

```rust
let context = server.context().clone();
tokio::spawn(async move {
    context.subscribe_stopping().await;
    // 回收连接池、定时器等资源
});
```

- 长连接（SSE、WebSocket 等）会延长优雅关闭的等待时间。
- 服务只启动一次；`run()` 结束后再次调用仍返回错误。

## 路由

`WebRouter::new(wrapper, interceptors)` 在构造时通过 `framework_web::web_api_iter()` 收集所有已注册路由，
按 `WebMethod` 与 `path` 建索引，注入的拦截器交给 `WebInterceptors` 按 `order()` 升序稳定排序。查找时忽略路径两端的 `/`，
未命中返回 `None`。

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
两层 task-local 作用域，再查找路由并执行拦截器与路由：

1. 查找路由，结果为 `Option<Arc<WebRoute>>`。
2. 把路由与上下文交给 `WebInterceptors::execute` 依次执行拦截器：`should_intercept` 返回 `false` 时跳过该拦截器；
   `intercept` 返回 `Next` 时继续下一个拦截器，返回 `Respond` 时直接以该响应结束请求，
   返回 `Redirect` 时用其路由替换当前路由，后续拦截器与路由都使用替换后的值。
3. 执行结果中仍没有路由时返回 404，否则交由 `WebRouteWrapper` 执行。

拦截器在两层 task-local 作用域内运行，可通过 `use_web()`、`use_axum()` 获取请求与服务信息。

## 拦截器

拦截器实现 `framework_web::WebInterceptor`，结果类型 `WebInterceptResult` 与语义见
[framework-web](../framework-web/README.md#拦截器)。

`AxumBuilder::push_interceptor(interceptor)` 逐个追加，`AxumServer::run` 启动时把这些拦截器交给
`WebRouter`，由 `WebInterceptors` 按 `order()` 升序排序并统一执行。需要自行组装时使用
`build_router(wrapper, interceptors)`，参数与 `WebRouter::new` 一致。

## 分发流程

`dispatch` 作为 axum 的 fallback 处理所有请求：

处于停止流程（`Stopping` / `Stopped`）的请求直接返回 503，响应体为
`{"code":503,"message":"Service Unavailable"}`，并附加 `connection: close`；不读取请求体、不进入路由，
但仍套用 CORS 与 `x-trace-id`。

1. 读取 `x-trace-id` 请求头，为空时用 `next_id()` 生成雪花 ID。
2. 收集请求头（键名转小写）与查询参数（保留原大小写），读取请求体，上限 16 MiB。
3. 组装 `WebRequest`：`scheme` 缺失时取 `http`，`authority` 缺失时取 `host` 请求头，
   `client_ip` 取连接对端 IP。
4. `OPTIONS` 请求直接返回 `WebResponse::empty()`（204），不进入路由查找。
5. 其他请求在 `catch_panic` 中执行 `WebRouter::invoke`，panic 与错误都转为错误响应。
6. 应用 CORS 响应头，写入 `x-trace-id`，转换为 axum `Response`。

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

`build_router(wrapper, interceptors)` 创建 `WebRouter`；`build_router_with(router, context, cors)`
把路由组装为 axum `Router`，并挂载统一的 `fallback` 分发。

# framework-http

统一的 HTTP 请求、下载与重试：客户端按需配置代理，支持任意方法请求与按方法快捷调用，
下载先探测远端文件是否支持分段，再决定分片并行还是顺序拉取。
请求头与响应头统一使用 `framework_core::MultiStringValue`。

## feature

| feature | 作用 |
|---------|------|
| `reqwest`（默认关闭） | reqwest 后端，未启用 `wreq` 时使用 |
| `wreq`（默认关闭） | 改用 wreq 后端，模拟 Chrome 的 TLS 指纹；与 `reqwest` 同时启用时使用本后端 |

```toml
# reqwest 后端
framework-http = { workspace = true, features = ["reqwest"] }

# wreq 后端：不再编译 reqwest
framework-http = { workspace = true, features = ["wreq"] }
```

后端 feature 默认全部关闭，两个都不启用时直接编译报错；两个都启用时使用 wreq 后端。

两个后端对外行为一致，仅支持非 wasm 目标。客户端超时固定 60 秒，最多跟随 10 次重定向，
并开启 gzip、brotli、zstd、deflate 解压。

## 客户端

```rust
use framework_http::HttpClient;

// 完全不使用代理，忽略环境变量与系统代理设置
let client = HttpClient::new();

// 所有请求都走指定代理，地址非法时报错
let proxied = HttpClient::with_proxy("socks5://127.0.0.1:1080")?;

// 按环境变量与操作系统代理设置选择代理
let system = HttpClient::with_system_proxy()?;
# Ok::<(), anyhow::Error>(())
```

| 构造器 | 说明 |
|--------|------|
| `HttpClient::new()` | 进程级共享的默认客户端，不使用任何代理 |
| `HttpClient::with_proxy(proxy)` | 新建客户端，所有请求走 `proxy`，支持 `http://`、`https://`、`socks5://` |
| `HttpClient::with_system_proxy()` | 新建客户端，读取 `HTTP_PROXY`、`HTTPS_PROXY`、`ALL_PROXY`、`NO_PROXY` 与系统代理设置 |

`HttpClient` 可以克隆，克隆出的副本共用同一份连接池。

## 原始方法

`request` 是原始入口：方法由 `HttpMethod` 指定，请求内容由 `HttpRequest` 完整描述。

```rust
use framework_http::{HttpClient, HttpMethod, HttpRequest};

let client = HttpClient::new();
let response = client
    .request(
        HttpMethod::Put,
        HttpRequest::new("https://example.com/api/item")
            .with_content_type("application/json")
            .with_body(r#"{"name":"demo"}"#),
    )
    .await?;
# Ok::<(), anyhow::Error>(())
```

| 方法 | 说明 |
|------|------|
| `request(method, request)` | 任意 `HttpMethod` |
| `get` / `post` / `put` / `patch` / `delete` / `head` / `options` | 把方法固定下来，参数仍是完整的 `HttpRequest` |

`HttpMethod` 取值 `Get`、`Post`、`Put`、`Patch`、`Delete`、`Head`、`Options`，
`as_str()` 返回协议中书写的方法名。

### HttpRequest

| 方法 | 说明 |
|------|------|
| `HttpRequest::new(url)` | 由地址构建，`url` 为必填 |
| `with_body(body)` | 设置请求体，接受任何 `Into<Bytes>` |
| `with(name, value)` | 写入一个请求头，同名覆盖 |
| `with_headers(&headers)` | 写入多个请求头，同名保留多条 |
| `with_ua` / `with_content_type` / `with_accept` / `with_authorization` | 常用请求头的快捷写入 |
| `has_ua()` | 是否已设置 `user-agent` |

请求头与响应头都是 `MultiStringValue`，`headers` 字段可直接读写。

## 快捷方法

只给地址（必要时给请求体）时用快捷方法：
只给地址（必要时给请求体）时用快捷方法：

```rust
let response = client.fetch_get("https://example.com/index.html").await?;
let response = client.fetch_post("https://example.com/api", "raw body").await?;
```

| 方法 | 说明 |
|------|------|
| `fetch_get(url)` | GET，响应体整体读入内存 |
| `fetch_post(url, body)` | POST，响应体整体读入内存 |
| `stream_get(url)` | GET，返回流式响应 |
| `stream_post(url, body)` | POST，返回流式响应 |

## 响应

| 方法 | 说明 |
|------|------|
| `is_success()` | 状态码是否 2xx |
| `string()` | 响应体转字符串，非法 utf-8 用替换字符 |
| `header(name)` | 响应头第一个值 |
| `content_type()` / `mime_type()` / `content_length()` | 常用响应头 |
| `is_empty()` / `len()` | 响应体是否为空、字节数 |

`status`、`body`、`headers` 为公开字段。

## 流式响应

响应体较大或需要边读边处理时用流式接口。`stream` 是原始入口，接收完整的 `HttpRequest` 并以 GET 发送；
`stream_get` / `stream_post` 是只给地址的快捷方法，三者的返回值都是 `HttpStreamResponse`：

```rust
use framework_http::{HttpClient, HttpRequest};

let client = HttpClient::new();
let mut response = client
    .stream(HttpRequest::new("https://example.com/big.bin"))
    .await?;

while let Some(chunk) = response.next().await {
    println!("{} 字节", chunk?.len());
}
```

| 成员 | 说明 |
|------|------|
| `status` / `headers` | 公开字段，状态码与响应头 |
| `is_success()` | 状态码是否 2xx |
| `header(name)` / `content_type()` / `mime_type()` / `content_length()` | 常用响应头 |
| `next()` | 读取下一块响应体，流结束返回 `None` |

状态码与响应头在拿到响应时即可读取；读流过程中出错会以 `Err` 从 `next()` 返回，不会中断已读到的内容。

## 下载

```rust
use framework_http::{DEFAULT_THREADS, HttpClient};

let client = HttpClient::new();
let outcome = client
    .download("https://example.com/big.bin", "/tmp/big.bin", DEFAULT_THREADS)
    .await?;

println!("{} 字节，分片：{}", outcome.written, outcome.chunked);
# Ok::<(), anyhow::Error>(())
```

| 方法 | 说明 |
|------|------|
| `probe(url)` | 探测远端文件大小、是否支持分段与 `etag` |
| `download(url, path, threads)` | 探测后下载，`threads` 大于 1 且文件不小于 1 MiB 时分片并行 |
| `download_probed(url, path, threads, probe)` | 用已有探测结果下载，避免同一个地址被探测两次 |
| `download_sequential(url, path)` | 不探测、不分片，直接顺序下载 |

`RemoteProbe` 字段为 `length`（无法确认时为 `None`）、`ranged`、`etag`，
`chunkable(threads)` 判断是否值得分片；`DownloadOutcome` 字段为 `written` 与 `chunked`。

探测先用 HEAD，HEAD 不可用时退回一次 `Range: bytes=0-0` 的 GET，两者都拿不到信息时按单点顺序下载。
分片下载会先按最终大小预分配文件，各分片写入自己的偏移；某块中断时只补没写完的部分，
重试次数用尽才报错。

## 重试

重试只作用在下载与探测发出的单个请求上，不会重放调用方发起的业务请求：

- 状态码 429、502、503、504 与请求本身失败都会重试，最多 4 次（含首次）；
- 响应带 `retry-after` 时按它等待，否则按 400 毫秒起的指数退避，单次等待上限 8 秒。

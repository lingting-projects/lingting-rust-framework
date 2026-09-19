# lingting-rust-framework

Rust 业务框架，按 crate 拆分职责：基础类型与运行时能力、Web 层运行时与 axum 适配，
以及一组支撑属性宏与 TypeScript 代码生成的 proc-macro crate。

## crate 一览

每个子 crate 的用途、用法与 feature 说明都在各自的 README 中，点击名称查看：

### 运行时

| crate | 说明 |
|-------|------|
| [framework-core](framework-core/README.md) | 统一响应 `R`、分页类型、雪花 ID、精确金额、应用目录、日志初始化、多值映射 |
| [framework-datetime](framework-datetime/README.md) | Unix 毫秒时间戳与可选 NTP 校时，区分原生与浏览器 wasm 平台 |
| [framework-region](framework-region/README.md) | 国家、地区、电话前缀与联合国 M49 区域静态数据 |
| [framework-web](framework-web/README.md) | 请求/响应模型、请求上下文、错误分类与日志、参数提取、授权规则、路由模型 |
| [framework-web-axum](framework-web-axum/README.md) | 基于 axum 的服务器适配：请求分发、路由收集、CORS 与服务启动 |

### 宏与代码生成

| crate | 说明 |
|-------|------|
| [framework-proc-core](framework-proc-core/README.md) | 元数据结构与注册表、TypeScript 类型/枚举/API 导出、npm 包打包 |
| [framework-proc-auto](framework-proc-auto/README.md) | `auto_type`、`auto_enum`、`auto_enum_impl`、`auto_enum_field` |
| [framework-proc-ts](framework-proc-ts/README.md) | `ts_api`：从函数签名收集 TypeScript 接口元数据 |
| [framework-proc-web](framework-proc-web/README.md) | `web_api` 及 HTTP 方法简化宏，展开为 `WebRoute` 构造函数 |

## 依赖方向

```text
framework-proc-core ──> framework-proc-auto
                    └──> framework-proc-ts

framework-datetime ─┐
framework-proc-auto ─┤
framework-proc-core ─┼──> framework-core ──> framework-web ──> framework-web-axum
framework-proc-ts ───┘                          ▲
                                                │
framework-proc-web ─────────────────────────────┘

framework-region（独立 crate，无内部依赖）
```

- `framework-core` 重导出 `framework-datetime` 的全部接口。
- `framework-web` 重导出 `framework-proc-web` 的宏。
- `collect` feature 用于开启基于 `inventory` 的静态注册，需要在最终二进制所在的 crate 上启用。

## 环境要求

- Rust 1.98.0（见 `rust-version`），edition 2024。

## 开发

```bash
cargo check --workspace
cargo clippy --fix --allow-dirty --allow-staged
cargo fmt
```

仓库根目录的 `.cargo/config.toml` 放宽了部分 lint（`non_upper_case_globals`、`unused_imports`、
`unused_variables`、`dead_code`、`non_snake_case`）。

## 许可证

[MIT](LICENSE)

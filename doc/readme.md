# 文档索引

本目录保存仓库的完整文档。每个子 crate 的用途、用法与 feature 说明都在各自的 `README.md` 中，点击名称查看。

## 运行时

| crate | 说明 |
|-------|------|
| [framework-bus](framework-bus/README.md) | 与 UI 无关的进程内通知总线：纯通知、类型即事件、边沿触发与合并 |
| [framework-core](framework-core/README.md) | 统一响应 `R`、分页类型、雪花 ID、精确金额、应用目录、日志初始化、多值映射 |
| [framework-datetime](framework-datetime/README.md) | Unix 毫秒时间戳与可选 NTP 校时，区分原生与浏览器 wasm 平台 |
| [framework-region](framework-region/README.md) | 国家、地区、电话前缀与联合国 M49 区域静态数据 |
| [framework-startup](framework-startup/README.md) | 开机自启的注册、注销与启停，区分系统级与用户级 |
| [framework-system](framework-system/README.md) | 提权判断 `is_elevated` 与 `std::process::Command` 扩展 `CommandExt` |
| [framework-web](framework-web/README.md) | 请求/响应模型、请求上下文、错误分类与日志、参数提取、授权规则、路由模型 |
| [framework-web-axum](framework-web-axum/README.md) | 基于 axum 的服务器适配：请求分发、路由收集、CORS 与服务启动 |
| [framework-timer](framework-timer/README.md) | 基于 Tokio 的进程内定时任务运行时与全局注册表 |

## 宏与代码生成

| crate | 说明 |
|-------|------|
| [framework-proc-core](framework-proc-core/README.md) | 元数据结构与注册表、TypeScript 类型/枚举/API 导出与 npm 包打包、Rust crate 生成 |
| [framework-proc-auto](framework-proc-auto/README.md) | `auto_type`、`auto_enum`、`auto_enum_impl`、`auto_enum_field` |
| [framework-proc-web](framework-proc-web/README.md) | `web_api` 及 HTTP 方法简化宏，展开为 `WebRoute` 构造函数 |
| [framework-proc-timer](framework-proc-timer/README.md) | `#[timer]` 属性宏，把 async 函数注册为定时任务 |

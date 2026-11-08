# 开发与发布

## 环境要求

| 项 | 要求 |
|----|------|
| Rust | 1.98.0（workspace 的 `rust-version`） |
| edition | 2024 |
| Shell | `scripts/*.sh` 需要 bash，Windows 下用 Git Bash |
| 发布凭据 | 发布到 crates.io 需要 `CARGO_REGISTRY_TOKEN` |

Windows / Linux / macOS 均为支持目标；`framework-datetime` 额外支持浏览器 wasm 目标，
相关平台差异由 crate 内部的 `cfg` 分支处理。workspace 中部分依赖是平台限定的
（`libc`、`windows-sys`、`sysinfo`、`js-sys`/`web-time` 等），只会在对应平台参与编译。

## 常用命令

```bash
cargo check --workspace --features framework-http/reqwest
cargo fmt
cargo clippy --fix --allow-dirty --allow-staged --all-features
```

- `framework-http` 的后端 feature 默认全部关闭，workspace 整体检查必须显式启用一个后端；
  只检查其他目标时也照此带上 `--features framework-http/reqwest`（或 `framework-http/wreq`）。
- 只检查部分 crate 时用 `-p <crate>`，例如 `cargo check -p framework-web`。
- 仓库内现有的测试代码位于 `framework-bus/tests/`，需要时用 `cargo test -p framework-bus` 运行。
- `.cargo/config.toml` 只承载 `[env]` 与 `[build] rustflags`，当前 `rustflags` 为空。

## feature

默认关闭全部非必要 feature，按需在最终二进制所在 crate 上逐层开启。完整说明见
[workspace.md](workspace.md#feature-传递)，常用组合：

```toml
# 需要 HTTP 服务 + 静态路由收集（framework-web-axum 默认即开启 collect）
framework-web-axum = { workspace = true }

# 需要生成 TypeScript 类型与接口
framework-web-axum = { workspace = true, features = ["ts"] }

# 需要 HTTP 客户端，reqwest 后端（wreq 后端换成 features = ["wreq"]）
framework-http = { workspace = true, features = ["reqwest"] }

# 需要定时任务
framework-timer = { workspace = true }

# 需要 NTP 校时（异步版包含 ntp）
framework-datetime = { workspace = true, features = ["tokio"] }
```

## 版本与 tag

版本号只在根 `Cargo.toml` 维护（`[workspace.package] version` 与各 `framework-*` 依赖的
`version`），子 crate 全部用 `version.workspace = true`。

```bash
# 改版本 -> 更新 Cargo.lock -> commit -> 打 tag v<version> -> 推送
bash scripts/tag.sh 26.8.142

# 只改版本并更新 lock，不执行任何 git 操作
bash scripts/tag.sh -d 26.8.142
```

`tag.sh` 需要版本号形如 `26.8.142`，tag 为 `v<version>`。

## 发布

```bash
bash scripts/publish.sh
```

`publish.sh` 会先校验根 `Cargo.toml` 里所有 `framework*` 依赖的版本与基准版本一致，
不一致直接中止；通过后调用 `cargo workspaces publish --publish-as-is --locked`，
逐个 crate 发布，并留出 15 秒间隔避开新 crate 的并发限制。

CI 见 `.github/workflows/publish.yml`：推送 `v*` tag 或手动触发，工作流会切到 `release`
分支后执行 `scripts/publish.sh`（cargo-workspaces 在 tag 触发的 CI 中不能直接使用，
所以先建分支再发布）。发布脚本在本地执行前需要已登录 crates.io 或设置
`CARGO_REGISTRY_TOKEN`。

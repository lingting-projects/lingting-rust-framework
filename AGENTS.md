# Agent 工作说明

本文件面向在仓库内工作的 AI Agent。目标：不依赖重新扫描目录即可定位信息，
并且改动落地时同步维护文档。

## 先读文档，再动代码

| 需要的信息 | 看哪个文档 |
|-----------|-----------|
| 仓库结构、crate 分层与依赖方向 | [workspace.md](workspace.md) |
| 环境要求、常用命令、feature、版本与发布 | [develop.md](develop.md) |
| 某个 crate 的用途、用法、参数与 feature | [doc/readme.md](doc/readme.md) 中对应目录的 `README.md` |
| 项目介绍与文档总入口 | [README.md](README.md) |

按索引直接读对应文件，不要为了“了解项目”而递归列举目录或批量读取全部 crate。
定位具体实现时再进入 `framework-*/src/`，并优先按符号 grep 而不是全量读文件。

## 结构速查

- workspace 成员在根 `Cargo.toml` 的 `members` 中，公共元数据在 `[workspace.package]`，
  共享依赖版本在 `[workspace.dependencies]`。
- 每个 crate 目录只有 `Cargo.toml` 与 `src/`；对外文档不在 crate 目录内，
  而是 `doc/<crate>/README.md`，由 crate 的 `readme` 字段指过去。
- `target/`、`.plan/` 是本地产物，不入版本库。

## 文档同步要求

改动涉及下列内容时，必须在同一次改动中更新对应文档，不允许留到后续：

| 改动内容 | 需要同步 |
|---------|---------|
| 新增 / 删除 / 改名 workspace crate，或调整 crate 之间的依赖 | [workspace.md](workspace.md)：分层表、依赖方向图、crate 文档索引；同时新增或删除 `doc/<crate>/README.md` 并更新 [doc/readme.md](doc/readme.md) |
| 新增、删除或调整 feature 及其默认值、传递关系 | [workspace.md](workspace.md) 的 feature 表；必要时更新对应 `doc/<crate>/README.md` |
| 仓库顶层目录、`scripts/`、CI 工作流发生变化 | [workspace.md](workspace.md) 的目录结构 |
| 环境要求、常用命令、版本与发布流程变化 | [develop.md](develop.md) |
| crate 的公开 API、宏参数、行为语义变化 | 对应 `doc/<crate>/README.md` |
| 文档入口或文档文件本身增删 | [README.md](README.md) 的文档索引 |

`doc/` 下各 `README.md` 是本仓库的唯一权威说明，代码注释不替代它们。

## 改动约束

- 只改完成任务所必需的部分，保持现有代码风格，不顺带优化无关代码。
- 不新增依赖或扩大 feature 范围；确有必要时先说明理由。
- 未经明确要求不运行测试、不创建测试代码或临时脚本。
- 提交前自查：本次改动是否触及上表任一项，若有则文档是否已同步。

---
name: merge-upstream-openfang
description: 合并上游 (up/main) 时如何解决典型冲突并保留双方功能。用于 pro 与上游同步后的冲突处理。
---

# 合并上游 OpenFang 冲突解决

## 何时使用

- 执行 `git merge up/main` 或从上游同步后出现冲突
- 需要**保留双方功能**（本分支 + 上游），而不是二选一

## CLI 结构说明（无需在合并里改）

- `crates/openfang-cli/src/main.rs` 已拆分为入口 + 子命令分发。
- 具体实现位于 `src/cmd/` 下各模块：`agent`、`config`、`hand`、`init`、`integration`、`model`、`system`、`telos`、`workflow`。
- 合并时若只有 `main.rs` 冲突，保留「入口解析 + 调用 `cmd::*`」的结构即可；功能改动尽量在 `cmd/*.rs` 里做。

## 合并后自检

1. `cargo build --workspace` 通过。
2. `cargo nextest run --workspace`（或 `cargo test`）通过。
3. `git add` 已解决的文件后完成 `git commit`。
4. commit message template: `chore: Merge [current version]~[up/main version] updates from the upstream main branch`
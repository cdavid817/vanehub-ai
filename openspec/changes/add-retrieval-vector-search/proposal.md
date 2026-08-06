## Why

`agent-cross-session-memory` 目前按时间倒序、在 4000 字符预算内把记忆注入 system prompt。
`api_process_adapter.rs:909-914` 的注释写明这是 "real retrieval 的有界替代品"，并记录 design.md
刻意推迟了 vector search/embeddings。会话一多，最相关的记忆就会被更晚但无关的记忆挤出预算。
本提案接替那个被推迟的决定。

## What Changes

- 新建 `retrieval` 对等上下文：自持 `retrieval_documents` 表，同一行同时承载内容、f32 向量与 FTS5 影子索引。
- 索引以差集协调（reconcile）为真源、后台异步补齐，保存记忆的路径不因 embedding 网络失败而失败。
- 检索为向量 + FTS5 双路召回、RRF 融合；任一路不可用即降级返回，不报错。
- 新增模型可主动调用的 `recall` 工具，scope 由运行时注入而非模型指定。
- OnePiece 配置页新增"检索"区块：来源 Profile、embedding 模型、索引状态、重建索引。
- **Non-goal（明确不做）**：分块、rerank 模型、ANN 索引、embedding 模型自动选择、改动现有 recency 注入链路、索引 `messages` 与本地文件。

## Capabilities

### New Capabilities
- `retrieval-vector-search`：检索内核、索引流水线、降级语义、`recall` 工具契约、检索配置。

### Modified Capabilities
- `agent-cross-session-memory`：记忆被删除时必须撤销其检索索引；现有 recency 注入行为本期不变。

## Impact

- 新增 SQLite 迁移 43 `retrieval-vector-index`（含 FTS5 虚拟表与三个 trigger）。
- Rust：新增 `contexts/retrieval/`、`bootstrap/retrieval.rs`、`commands/retrieval/`；
  修改 `contexts/mod.rs`、`platform/database/migrations.rs`、`agent_runtime` 的
  `api.rs` / `application/service.rs` / `application/tool_catalog.rs` /
  `infrastructure/api_process_adapter.rs`。
- 前端：`agent-service.ts` 新增 5 个方法，`tauri-agent-client.ts` 与 `web-agent-client.ts` 同步实现。
- Web/mock runtime 保证契约形状与可观察行为对等，不保证排序算法等价。
- 未配置 embedding 时全部现有行为不变：`recall` 不注册，recency 注入照常。

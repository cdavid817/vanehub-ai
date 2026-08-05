# OnePiece 向量检索 · 第 1 期实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 给 OnePiece 的跨会话记忆装上"向量 + 关键词"混合检索，并以模型可主动调用的 `recall` 工具暴露出来。

**Architecture:** 新建 `retrieval` 对等上下文，自持 `retrieval_documents` 表（内容 + f32 BLOB 向量 + FTS5 影子表）。后台 worker 以"差集协调（reconcile）"为真源异步补齐 embedding，保存路径完全不感知索引。检索时向量路（Rust 暴力余弦）与关键词路（FTS5 bm25）双路召回后用 RRF 融合，任一路失败即降级而非报错。embedding 端点与凭据由 `agent_runtime` 通过两条新增跨上下文 api 提供，`retrieval` 不知道 Profile 与凭据存储的存在。

**Tech Stack:** Rust / Tauri 2.x / rusqlite（SQLite + FTS5 trigram）/ sha2 / uuid / reqwest（复用 `platform/network`）/ React 19 + TypeScript strict / Tailwind / vitest / Playwright

**依据规范：** `docs/superpowers/specs/2026-08-05-onepiece-vector-search-design.md`（下称"设计文档"，本计划中的 §N 均指其章节）

---

## Global Constraints

以下约束来自 `AGENTS.md` 与设计文档，**每个任务都隐含包含本节全部内容**：

- 包管理只用 **npm**。仓库有 `package-lock.json`，禁止 pnpm/yarn（`pnpm-lock.yaml` 若出现是污染，删掉不要提交）。
- TypeScript strict：禁止 `any`，禁止 `// @ts-ignore`（必须绕过时用 `// @ts-expect-error` 并写明原因）。
- React 只用函数组件 + Hooks，禁止 class component，**单文件不超过 300 行**。
- 样式只用 Tailwind：不写内联 `style`，不引入 styled-components / CSS Modules / 其他 UI 组件库。
- 状态管理只用 React 内置 state/context。
- **React 组件禁止直接调用 Tauri `invoke()`**，必须经 `src/services/agent-service.ts` 的服务接口。
- `src/services/tauri-agent-client.ts` 与 `src/services/web-agent-client.ts` **必须保持接口一致，新增能力两处同时改**。
- Rust：跨 Tauri command 边界的错误必须是 `Result<T, String>` 或自定义 error enum；`unwrap()`/`expect()` 仅限测试代码。
- 上下文边界：`agent_runtime` 只通过 `retrieval::api` 交互，**不 import** `retrieval` 的 repository 或 infrastructure；反向同理（设计文档 §4.3）。
- 日志遵循 `openspec/specs/unified-log-management/spec.md`。**绝不落盘**：记忆内容、query 原文、API key、provider 响应体。query 只记长度与哈希（设计文档 §8.2）。
- 注释只写"为什么这样做"，不写代码翻译式注释。
- 可调常量集中定义在一处，不散落调用点（设计文档 §5.2）。本期四个：批大小 `32`、轮询 `5` 分钟、重试 `5` 次、截断 `8000` 字符。
- 每个任务结束时必须能独立跑通其自身测试；全量验收命令见 Task 17。

---

## File Structure

### 新建（Rust）

```
src-tauri/src/contexts/retrieval/
├─ mod.rs                                  # 挂 api/application/domain/infrastructure 四个子模块
├─ api.rs                                  # 唯一跨上下文出口：index/remove/search/配置/状态/重建
├─ domain/
│  ├─ mod.rs
│  ├─ document.rs                          # RetrievalDocument / SourceKind / IndexState / FailureCategory / document_id / content_hash
│  ├─ query.rs                             # RetrievalQuery / RetrievalScope / ScoredHit / MatchedVia / Degradation / escape_fts_query
│  ├─ vector.rs                            # f32 BLOB 编解码 + 余弦
│  ├─ fusion.rs                            # RRF 融合（纯函数，不碰 I/O）
│  └─ error.rs                             # RetrievalError
├─ application/
│  ├─ mod.rs
│  ├─ ports.rs                             # EmbeddingPort / EmbeddingEndpointPort / RetrievalDocumentRepository / RetrievalConfigurationRepository / MemorySourcePort
│  ├─ indexing_service.rs                  # reconcile + 批处理 worker 一轮
│  └─ search_service.rs                    # 双路召回 + RRF + 回查 + 降级
└─ infrastructure/
   ├─ mod.rs
   ├─ schema.rs                            # 迁移 42 的建表/索引/FTS/trigger
   ├─ sqlite_repository.rs                 # RetrievalDocumentRepository 实现
   ├─ configuration_repository.rs          # RetrievalConfigurationRepository 实现
   └─ openai_embedding_adapter.rs          # EmbeddingPort 的 openai-compatible HTTP 实现
```

```
src-tauri/src/bootstrap/retrieval.rs       # 装配 + 后台 worker + EmbeddingEndpointPort 的跨上下文适配器
src-tauri/src/commands/retrieval/          # 每命令一文件：get/save 配置、列 embedding 模型、查状态、重建
```

### 修改（Rust）

- `src-tauri/src/contexts/mod.rs` — 注册 `retrieval` 模块
- `src-tauri/src/platform/database/migrations.rs` — 注册迁移 42
- `src-tauri/src/contexts/agent_runtime/application/service.rs:71-87` — `is_chat_model` 改为从统一的模型类别判定派生
- `src-tauri/src/contexts/agent_runtime/api.rs` — 新增 `resolve_embedding_endpoint` / `list_embedding_models`
- `src-tauri/src/contexts/agent_runtime/application/tool_catalog.rs` — 新增 `RECALL_TOOL_NAME`、`recall_tool_definition()`、risk tier
- `src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs` — `resolve_tool_catalog` 注入条件性 `recall`；`execute_tool_call` 路由 `recall`
- `src-tauri/src/commands/agent_runtime/delete_agent_memory.rs` — 删除后撤销索引
- `src-tauri/src/bootstrap/mod.rs`、`src-tauri/src/lib.rs` — 装配与命令注册

### 修改（前端）

- `src/services/agent-service.ts` — 5 个新方法（设计文档 §7.4）
- `src/services/tauri-agent-client.ts`、`src/services/web-agent-client.ts` — 同时实现
- `src/settings/pages/agents/onepiece-retrieval-section.tsx`（**新建**）— 检索配置区块，独立文件避免 `onepiece-configuration-panel.tsx`（现 133 行）膨胀过 300 行
- `src/settings/pages/agents/onepiece-configuration-panel.tsx` — 挂载新区块
- `src/types/agent.ts` — 新增 `RetrievalConfiguration` / `RetrievalIndexStatus` / `EmbeddingModelOption`
- `tests/e2e/onepiece-retrieval.spec.ts`（**新建**）

---

### Task 1: OpenSpec 提案

`AGENTS.md` 规定：任何新功能必须先在 `openspec/changes/` 起 proposal 并通过校验，**再动代码**。本任务不写任何产品代码。

**Files:**
- Create: `openspec/changes/add-retrieval-vector-search/.openspec.yaml`
- Create: `openspec/changes/add-retrieval-vector-search/proposal.md`
- Create: `openspec/changes/add-retrieval-vector-search/design.md`
- Create: `openspec/changes/add-retrieval-vector-search/tasks.md`
- Create: `openspec/changes/add-retrieval-vector-search/specs/retrieval-vector-search/spec.md`
- Create: `openspec/changes/add-retrieval-vector-search/specs/agent-cross-session-memory/spec.md`

**Interfaces:**
- Consumes: 设计文档全文
- Produces: change id `add-retrieval-vector-search`，后续所有任务的 spec 依据

- [ ] **Step 1: 建目录与 `.openspec.yaml`**

```yaml
schema: spec-driven
created: 2026-08-05
```

- [ ] **Step 2: 写 `proposal.md`**

四个 H2 小节，格式照 `openspec/changes/add-gemini-cli-terminal-usage-tracking/proposal.md`：

```markdown
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

- 新增 SQLite 迁移 42 `retrieval-vector-index`（含 FTS5 虚拟表与三个 trigger）。
- Rust：新增 `contexts/retrieval/`、`bootstrap/retrieval.rs`、`commands/retrieval/`；
  修改 `contexts/mod.rs`、`platform/database/migrations.rs`、`agent_runtime` 的
  `api.rs` / `application/service.rs` / `application/tool_catalog.rs` /
  `infrastructure/api_process_adapter.rs`。
- 前端：`agent-service.ts` 新增 5 个方法，`tauri-agent-client.ts` 与 `web-agent-client.ts` 同步实现。
- Web/mock runtime 保证契约形状与可观察行为对等，不保证排序算法等价。
- 未配置 embedding 时全部现有行为不变：`recall` 不注册，recency 注入照常。
```

- [ ] **Step 3: 写 `design.md`**

把 `docs/superpowers/specs/2026-08-05-onepiece-vector-search-design.md` 的 §3（关键决策表）、§4.2（数据模型）、§4.3（跨上下文依赖方向）、§6.1（降级矩阵）搬进来。**不要写成"见另一份文档"**——OpenSpec 归档后必须自包含。

- [ ] **Step 4: 写新能力 delta spec**

`specs/retrieval-vector-search/spec.md`，以 `## ADDED Requirements` 开头。至少覆盖下列 requirement，每条至少一个 `#### Scenario:`（格式照 `add-gemini-cli-terminal-usage-tracking/specs/usage-statistics/spec.md`）：

```markdown
## ADDED Requirements

### Requirement: Retrieval scope is runtime-injected
The system SHALL derive retrieval scope from the session runtime and SHALL NOT accept agent id or workspace folder as model-supplied tool input.

#### Scenario: Model cannot widen its own scope
- **WHEN** the model invokes the recall tool
- **THEN** the system SHALL scope the search to the session's own agent id and workspace folder
- **AND** the tool input schema SHALL NOT expose any scope parameter

### Requirement: Retrieval failure never fails generation
The system SHALL return a successful tool result describing unavailability when retrieval fails, and SHALL NOT surface retrieval failure as a generation error.

#### Scenario: Embedding provider unreachable during search
- **WHEN** query embedding fails while retrieval is configured
- **THEN** the system SHALL return keyword-only results marked `degraded: keyword_only`

#### Scenario: Keyword path fails
- **WHEN** the FTS5 query fails
- **THEN** the system SHALL return vector-only results marked `degraded: vector_only`

#### Scenario: Both paths yield nothing
- **WHEN** neither path returns a hit
- **THEN** the system SHALL return an empty result list and SHALL NOT report an error

### Requirement: Saving a memory never depends on indexing
The system SHALL persist an agent memory without requiring its retrieval index entry to be written in the same operation.

#### Scenario: Indexing backend unavailable at save time
- **WHEN** a memory is saved while the embedding provider is unreachable
- **THEN** the save SHALL succeed
- **AND** the memory SHALL become searchable by keyword immediately and by vector once background indexing converges

### Requirement: Vector recall only compares same-model embeddings
The system SHALL restrict vector recall to rows whose stored embedding model equals the currently configured embedding model.

#### Scenario: Embedding model changed
- **WHEN** the configured embedding model differs from a row's stored embedding model
- **THEN** that row SHALL be excluded from vector recall
- **AND** that row SHALL remain reachable through the keyword path
- **AND** the system SHALL re-queue that row for background re-indexing

### Requirement: Retrieval tool is registered only when configured
The system SHALL offer the recall tool to the model only when an embedding source is configured.

#### Scenario: No embedding configured
- **WHEN** no embedding source is configured
- **THEN** the recall tool SHALL NOT appear in the tool catalog
- **AND** existing recency-based memory injection SHALL continue unchanged

### Requirement: Retrieval logging excludes sensitive content
The system SHALL NOT persist memory content, raw query text, credentials, or provider response bodies to logs.

#### Scenario: Query logged for diagnostics
- **WHEN** a retrieval executes
- **THEN** the system SHALL log only the query's length and hash alongside scope hash, candidate count, per-path hit counts, and duration

### Requirement: Web runtime contract parity
The Web/mock runtime SHALL expose the same retrieval contract shape and observable behavior as the desktop runtime, and SHALL NOT issue network requests.

#### Scenario: Web runtime search
- **WHEN** retrieval is invoked in the Web/mock runtime
- **THEN** it SHALL return the same result structure, the same degraded semantics, and treat empty results as success
- **AND** it MAY rank by a simple term-overlap score rather than reproducing vector similarity
```

- [ ] **Step 5: 写被修改能力的 delta spec**

`specs/agent-cross-session-memory/spec.md`：

```markdown
## ADDED Requirements

### Requirement: Deleting a memory revokes its retrieval index
The system SHALL revoke the retrieval index entry for a deleted memory, and SHALL NOT return deleted memories from retrieval even if an index entry survives.

#### Scenario: Memory deleted while index revocation fails
- **WHEN** a memory is deleted and its index revocation call fails
- **THEN** retrieval SHALL NOT return that memory, because results are resolved against the source table
- **AND** background reconciliation SHALL remove the orphaned index entry
```

- [ ] **Step 6: 写 `tasks.md`**

按本计划 Task 2–17 逐条列出，勾选框格式 `- [ ]`。

- [ ] **Step 7: 校验**

Run: `openspec validate add-retrieval-vector-search --strict`
Expected: 通过，无 error。

Run: `openspec validate --specs --strict`
Expected: 通过。

- [ ] **Step 8: 提交**

```bash
git add openspec/changes/add-retrieval-vector-search
git commit -m "spec: propose retrieval vector search phase 1"
```

---

### Task 2: `retrieval` 上下文骨架与领域类型

**Files:**
- Create: `src-tauri/src/contexts/retrieval/mod.rs`
- Create: `src-tauri/src/contexts/retrieval/domain/mod.rs`
- Create: `src-tauri/src/contexts/retrieval/domain/document.rs`
- Create: `src-tauri/src/contexts/retrieval/domain/error.rs`
- Modify: `src-tauri/src/contexts/mod.rs`

**Interfaces:**
- Produces: `SourceKind`、`IndexState`、`FailureCategory`、`RetrievalDocument`、`document_id(SourceKind, &str) -> String`、`content_hash(&str) -> String`、`RetrievalError`。Task 4 起全部依赖这些名字与签名。

- [ ] **Step 1: 写失败的测试**

`src-tauri/src/contexts/retrieval/domain/document.rs` 末尾：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_id_is_deterministic_and_namespaced_by_source_kind() {
        assert_eq!(
            document_id(SourceKind::AgentMemory, "mem-1"),
            "agent_memory:mem-1"
        );
        assert_eq!(
            document_id(SourceKind::AgentMemory, "mem-1"),
            document_id(SourceKind::AgentMemory, "mem-1")
        );
    }

    #[test]
    fn content_hash_is_stable_and_distinguishes_content() {
        let first = content_hash("Uses npm, not pnpm.");
        assert_eq!(first, content_hash("Uses npm, not pnpm."));
        assert_ne!(first, content_hash("Uses pnpm."));
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }

    #[test]
    fn source_kind_round_trips_through_its_persisted_form() {
        assert_eq!(SourceKind::AgentMemory.as_str(), "agent_memory");
        assert_eq!(SourceKind::parse("agent_memory"), Some(SourceKind::AgentMemory));
        assert_eq!(SourceKind::parse("nonsense"), None);
    }

    #[test]
    fn index_state_round_trips_through_its_persisted_form() {
        for state in [IndexState::Pending, IndexState::Indexed, IndexState::Failed] {
            assert_eq!(IndexState::parse(state.as_str()), Some(state));
        }
        assert_eq!(IndexState::parse("nonsense"), None);
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::domain::document`
Expected: FAIL —— 编译错误 `unresolved module` / `cannot find function document_id`。

- [ ] **Step 3: 写最小实现**

`src-tauri/src/contexts/retrieval/domain/document.rs`（测试模块保留在文件末尾）：

```rust
use sha2::{Digest, Sha256};

/// 索引来源类别。第 1 期只有 `AgentMemory`；第 2/3 期扩展 `session_message`、`workspace_file`。
/// 字符串形式是持久化格式，改动即破坏既有索引行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceKind {
    AgentMemory,
}

impl SourceKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::AgentMemory => "agent_memory",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "agent_memory" => Some(Self::AgentMemory),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexState {
    Pending,
    Indexed,
    Failed,
}

impl IndexState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Indexed => "indexed",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "indexed" => Some(Self::Indexed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// 决定重试策略：`Auth`/`InvalidRequest` 是确定性失败，重试只会烧配额（设计文档 §5.2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailureCategory {
    Auth,
    InvalidRequest,
    RateLimit,
    Network,
}

impl FailureCategory {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::InvalidRequest => "invalid_request",
            Self::RateLimit => "rate_limit",
            Self::Network => "network",
        }
    }

    pub(crate) fn is_retryable(self) -> bool {
        matches!(self, Self::RateLimit | Self::Network)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RetrievalDocument {
    pub(crate) id: String,
    pub(crate) source_kind: SourceKind,
    pub(crate) source_id: String,
    pub(crate) scope_agent_id: String,
    /// 空串哨兵表示"无工作区文件夹"，与 `agent_memories.folder` 一致（`memory_schema.rs:4-6`）。
    pub(crate) scope_folder: String,
    pub(crate) content: String,
    pub(crate) content_hash: String,
    pub(crate) index_state: IndexState,
    pub(crate) attempt_count: u32,
    pub(crate) embedding_model: Option<String>,
}

/// 确定性主键，与 `UNIQUE (source_kind, source_id)` 同源——reconcile 因此可以直接 upsert，
/// 不必先查后插。
pub(crate) fn document_id(source_kind: SourceKind, source_id: &str) -> String {
    format!("{}:{}", source_kind.as_str(), source_id)
}

pub(crate) fn content_hash(content: &str) -> String {
    bytes_to_hex(&Sha256::digest(content.as_bytes()))
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
```

`src-tauri/src/contexts/retrieval/domain/error.rs`：

```rust
/// 上下文内部错误。在 `api.rs` 边界转换；跨 Tauri command 边界按 AGENTS.md 转为 `Result<T, String>`。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RetrievalError {
    Storage(String),
    Embedding(String),
    NotConfigured,
    /// 两路召回都失败。与"两路都可用但没命中"必须是不同的结果：把"搜不了"报告成
    /// "没有"，会让模型据此断定用户从没提过某事。
    Unavailable,
}

impl std::fmt::Display for RetrievalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "retrieval storage error: {message}"),
            Self::Embedding(message) => write!(formatter, "embedding error: {message}"),
            Self::NotConfigured => write!(formatter, "retrieval is not configured"),
            Self::Unavailable => write!(formatter, "retrieval is temporarily unavailable"),
        }
    }
}
```

`src-tauri/src/contexts/retrieval/domain/mod.rs`：

```rust
pub(crate) mod document;
pub(crate) mod error;

pub(crate) use document::{
    content_hash, document_id, FailureCategory, IndexState, RetrievalDocument, SourceKind,
};
pub(crate) use error::RetrievalError;
```

`src-tauri/src/contexts/retrieval/mod.rs`：

```rust
//! 语义 + 关键词混合检索内核。第 1 期只索引跨会话记忆（`add-retrieval-vector-search`）。

pub(crate) mod domain;
```

`src-tauri/src/contexts/mod.rs` 的模块列表按字母序插入一行（在 `operations` 之后）：

```rust
pub(crate) mod retrieval;
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::domain::document`
Expected: PASS，4 个测试。

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml`
Expected: 无 warning。未被使用的 pub(crate) 项若触发 `dead_code`，本任务先给 `RetrievalDocument` 与 `FailureCategory` 加 `#[allow(dead_code)]`；**Task 5 的仓储 trait 签名会用上这两个类型，届时必须删除该属性**（留着它会长期掩盖真实的死代码）。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/contexts/retrieval src-tauri/src/contexts/mod.rs
git commit -m "feat(retrieval): add context skeleton and domain document types"
```

---

### Task 3: 纯算法——余弦、f32 BLOB 编解码、RRF、FTS 转义

三者都是无 I/O 纯函数，放在一个任务里：它们没有互相依赖，但共同构成"可以独立于数据库验证的算法内核"，一个 reviewer 一次看完最省事。

**Files:**
- Create: `src-tauri/src/contexts/retrieval/domain/vector.rs`
- Create: `src-tauri/src/contexts/retrieval/domain/fusion.rs`
- Create: `src-tauri/src/contexts/retrieval/domain/query.rs`
- Modify: `src-tauri/src/contexts/retrieval/domain/mod.rs`

**Interfaces:**
- Consumes: 无
- Produces:
  - `encode_embedding(&[f32]) -> Vec<u8>`、`decode_embedding(&[u8]) -> Option<Vec<f32>>`、`cosine_similarity(&[f32], &[f32]) -> Option<f32>`
  - `fuse_with_rrf(&[Vec<String>]) -> Vec<(String, f64)>`
  - `escape_fts_query(&str) -> String`
  - `RetrievalScope { agent_id: String, folder: String }`、`RetrievalQuery { text: String, scope: RetrievalScope, limit: usize }`、`MatchedVia`、`Degradation`、`ScoredHit`

- [ ] **Step 1: 写失败的测试**

`vector.rs` 末尾：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_round_trips_through_its_blob_form() {
        let original = vec![0.0_f32, 1.5, -2.25, 1e-7];
        let decoded = decode_embedding(&encode_embedding(&original)).expect("decodes");
        assert_eq!(decoded, original);
    }

    #[test]
    fn blob_with_a_length_that_is_not_a_multiple_of_four_is_rejected() {
        assert_eq!(decode_embedding(&[0, 0, 0]), None);
    }

    #[test]
    fn cosine_of_identical_vectors_is_one() {
        let similarity = cosine_similarity(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]).expect("similarity");
        assert!((similarity - 1.0).abs() < 1e-6, "got {similarity}");
    }

    #[test]
    fn cosine_of_orthogonal_vectors_is_zero() {
        let similarity = cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).expect("similarity");
        assert!(similarity.abs() < 1e-6, "got {similarity}");
    }

    #[test]
    fn cosine_of_opposite_vectors_is_negative_one() {
        let similarity = cosine_similarity(&[1.0, 2.0], &[-1.0, -2.0]).expect("similarity");
        assert!((similarity + 1.0).abs() < 1e-6, "got {similarity}");
    }

    #[test]
    fn cosine_rejects_dimension_mismatch_and_zero_vectors() {
        assert_eq!(cosine_similarity(&[1.0, 2.0], &[1.0]), None);
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 2.0]), None);
    }
}
```

`fusion.rs` 末尾：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn ids(ranking: &[(String, f64)]) -> Vec<&str> {
        ranking.iter().map(|(id, _)| id.as_str()).collect()
    }

    #[test]
    fn a_document_ranked_first_by_both_paths_wins() {
        let vector = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let keyword = vec!["a".to_string(), "c".to_string()];
        assert_eq!(ids(&fuse_with_rrf(&[vector, keyword]))[0], "a");
    }

    #[test]
    fn a_document_found_by_both_paths_beats_one_found_only_higher_by_a_single_path() {
        // b: 两路各第 2 名 → 2/62；a: 只有向量路第 1 名 → 1/61。两次中游胜过一次头名。
        let vector = vec!["a".to_string(), "b".to_string()];
        let keyword = vec!["c".to_string(), "b".to_string()];
        assert_eq!(ids(&fuse_with_rrf(&[vector, keyword]))[0], "b");
    }

    #[test]
    fn an_empty_path_does_not_affect_the_other_path_order() {
        let vector = vec!["a".to_string(), "b".to_string()];
        assert_eq!(ids(&fuse_with_rrf(&[vector, Vec::new()])), vec!["a", "b"]);
    }

    #[test]
    fn fusing_nothing_yields_nothing() {
        assert!(fuse_with_rrf(&[Vec::new(), Vec::new()]).is_empty());
    }

    #[test]
    fn ties_are_broken_deterministically_by_id() {
        let first = vec!["b".to_string(), "a".to_string()];
        let second = vec!["a".to_string(), "b".to_string()];
        assert_eq!(ids(&fuse_with_rrf(&[first, second])), vec!["a", "b"]);
    }
}
```

`query.rs` 末尾：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_query_becomes_a_quoted_phrase() {
        assert_eq!(escape_fts_query("npm not pnpm"), "\"npm not pnpm\"");
    }

    #[test]
    fn embedded_double_quotes_are_doubled_not_dropped() {
        assert_eq!(escape_fts_query("use \"npm\""), "\"use \"\"npm\"\"\"");
    }

    #[test]
    fn fts_operators_lose_their_syntactic_meaning() {
        for raw in ["a OR b", "a NEAR b", "prefix*", "col:value", "-excluded"] {
            let escaped = escape_fts_query(raw);
            assert!(escaped.starts_with('"') && escaped.ends_with('"'), "{escaped}");
            assert!(escaped.contains(raw), "{escaped} should carry {raw} verbatim");
        }
    }

    #[test]
    fn whitespace_only_queries_escape_to_an_empty_phrase() {
        assert_eq!(escape_fts_query("   "), "\"\"");
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::domain`
Expected: FAIL —— `cannot find function encode_embedding` 等。

- [ ] **Step 3: 写最小实现**

`vector.rs`：

```rust
/// 存储格式：f32 little-endian 连续字节。选它而不是 JSON 数组，是因为向量路要在候选集上做
/// 暴力扫描，反序列化开销直接进热路径。
pub(crate) fn encode_embedding(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub(crate) fn decode_embedding(bytes: &[u8]) -> Option<Vec<f32>> {
    if bytes.len() % 4 != 0 {
        return None;
    }
    Some(
        bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect(),
    )
}

/// 维度不一致或任一侧为零向量时返回 `None`——没有有意义的相似度可言，交由调用方跳过该候选，
/// 而不是伪造一个 0.0 混进排名。
pub(crate) fn cosine_similarity(left: &[f32], right: &[f32]) -> Option<f32> {
    if left.len() != right.len() || left.is_empty() {
        return None;
    }
    let mut dot = 0.0_f64;
    let mut left_norm = 0.0_f64;
    let mut right_norm = 0.0_f64;
    for (a, b) in left.iter().zip(right.iter()) {
        dot += f64::from(*a) * f64::from(*b);
        left_norm += f64::from(*a) * f64::from(*a);
        right_norm += f64::from(*b) * f64::from(*b);
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return None;
    }
    Some((dot / (left_norm.sqrt() * right_norm.sqrt())) as f32)
}
```

`fusion.rs`：

```rust
use std::collections::HashMap;

/// RRF 的平滑常数。60 是原论文的取值，作用是压低头部名次的边际优势，让"两路都中游"
/// 胜过"一路头名、另一路缺席"——这正是混合检索想要的行为。
const RRF_SMOOTHING: f64 = 60.0;

/// 输入是若干条已排好序的 id 列表（每条代表一路召回），输出按融合分降序。
/// 同分时按 id 升序，保证同样输入永远给出同样顺序——否则测试与 UI 都会闪。
pub(crate) fn fuse_with_rrf(rankings: &[Vec<String>]) -> Vec<(String, f64)> {
    let mut scores: HashMap<&str, f64> = HashMap::new();
    for ranking in rankings {
        for (index, id) in ranking.iter().enumerate() {
            let rank = index as f64 + 1.0;
            *scores.entry(id.as_str()).or_insert(0.0) += 1.0 / (RRF_SMOOTHING + rank);
        }
    }
    let mut fused: Vec<(String, f64)> = scores
        .into_iter()
        .map(|(id, score)| (id.to_string(), score))
        .collect();
    fused.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });
    fused
}
```

`query.rs`：

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetrievalScope {
    pub(crate) agent_id: String,
    /// 空串表示"无工作区文件夹"，与 `agent_memories.folder` 的哨兵一致。
    pub(crate) folder: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetrievalQuery {
    pub(crate) text: String,
    pub(crate) scope: RetrievalScope,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MatchedVia {
    Vector,
    Keyword,
    Both,
}

impl MatchedVia {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Vector => "vector",
            Self::Keyword => "keyword",
            Self::Both => "both",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Degradation {
    KeywordOnly,
    VectorOnly,
}

impl Degradation {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::KeywordOnly => "keyword_only",
            Self::VectorOnly => "vector_only",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScoredHit {
    pub(crate) source_id: String,
    pub(crate) content: String,
    pub(crate) created_at: String,
    pub(crate) score: f64,
    pub(crate) matched_via: MatchedVia,
}

/// 把整条 query 转义成**单个 FTS5 字符串字面量**。
///
/// 仓库里唯一的既有 FTS 消费方 `contexts/workspaces/infrastructure/output_search.rs:36-47`
/// 是把原始串直接塞进 `MATCH ?1` 的，只挡空串与超长。这里不能照抄：`recall` 的 query 由模型
/// 自由生成，含 `"` `*` `:` `-` `OR` `NEAR` 时 FTS5 会按查询语法解析，轻则语义跑偏，重则整条
/// 语句报错。转义成短语后，trigram tokenizer 下的子串匹配正是我们想要的行为。
pub(crate) fn escape_fts_query(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len() + 2);
    escaped.push('"');
    for character in raw.trim().chars() {
        if character == '"' {
            escaped.push('"');
        }
        escaped.push(character);
    }
    escaped.push('"');
    escaped
}
```

`domain/mod.rs` 追加：

```rust
pub(crate) mod fusion;
pub(crate) mod query;
pub(crate) mod vector;

pub(crate) use fusion::fuse_with_rrf;
pub(crate) use query::{
    escape_fts_query, Degradation, MatchedVia, RetrievalQuery, RetrievalScope, ScoredHit,
};
pub(crate) use vector::{cosine_similarity, decode_embedding, encode_embedding};
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::domain`
Expected: PASS，共 19 个测试（Task 2 的 4 个 + 本任务 15 个）。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/contexts/retrieval/domain
git commit -m "feat(retrieval): add cosine, RRF fusion, and FTS query escaping"
```

---

### Task 4: 迁移 42 与 schema

**Files:**
- Create: `src-tauri/src/contexts/retrieval/infrastructure/mod.rs`
- Create: `src-tauri/src/contexts/retrieval/infrastructure/schema.rs`
- Modify: `src-tauri/src/contexts/retrieval/mod.rs`
- Modify: `src-tauri/src/platform/database/migrations.rs`（在 `41 onepiece-provider-endpoints` 之后追加）

**Interfaces:**
- Consumes: 无
- Produces: `apply_retrieval_schema(&Connection) -> Result<(), DatabaseError>`，Task 5/6 的仓储依赖其建出的表

**核实过的现状：** 当前最新迁移是 41 `onepiece-provider-endpoints`（`migrations.rs:236`），所以 42 是下一个号。FTS 写法照搬 `apply_session_message_search_migration`（`migrations.rs:274-301`）的 external-content + 三 trigger + `tokenize='trigram'`。

- [ ] **Step 1: 写失败的测试**

`schema.rs` 末尾：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn migrated_connection() -> Connection {
        let connection = Connection::open_in_memory().expect("database");
        apply_retrieval_schema(&connection).expect("first apply");
        connection
    }

    #[test]
    fn schema_is_idempotent() {
        let connection = migrated_connection();
        apply_retrieval_schema(&connection).expect("second apply");

        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'retrieval_documents'",
                [],
                |row| row.get(0),
            )
            .expect("table count");
        assert_eq!(table_count, 1);
    }

    #[test]
    fn inserting_a_document_populates_the_fts_shadow_table() {
        let connection = migrated_connection();
        connection
            .execute(
                "INSERT INTO retrieval_documents
                 (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash, created_at, updated_at)
                 VALUES ('agent_memory:m1','agent_memory','m1','a','', 'uses npm not pnpm', 'h', 't', 't')",
                [],
            )
            .expect("insert");

        let hits: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM retrieval_documents_fts WHERE retrieval_documents_fts MATCH '\"npm\"'",
                [],
                |row| row.get(0),
            )
            .expect("fts count");
        assert_eq!(hits, 1);
    }

    #[test]
    fn updating_content_replaces_the_fts_entry() {
        let connection = migrated_connection();
        connection
            .execute(
                "INSERT INTO retrieval_documents
                 (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash, created_at, updated_at)
                 VALUES ('agent_memory:m1','agent_memory','m1','a','', 'uses npm', 'h', 't', 't')",
                [],
            )
            .expect("insert");
        connection
            .execute(
                "UPDATE retrieval_documents SET content = 'uses cargo' WHERE id = 'agent_memory:m1'",
                [],
            )
            .expect("update");

        let stale: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM retrieval_documents_fts WHERE retrieval_documents_fts MATCH '\"npm\"'",
                [],
                |row| row.get(0),
            )
            .expect("stale count");
        let fresh: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM retrieval_documents_fts WHERE retrieval_documents_fts MATCH '\"cargo\"'",
                [],
                |row| row.get(0),
            )
            .expect("fresh count");
        assert_eq!(stale, 0);
        assert_eq!(fresh, 1);
    }

    #[test]
    fn deleting_a_document_clears_its_fts_entry() {
        let connection = migrated_connection();
        connection
            .execute(
                "INSERT INTO retrieval_documents
                 (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash, created_at, updated_at)
                 VALUES ('agent_memory:m1','agent_memory','m1','a','', 'uses npm', 'h', 't', 't')",
                [],
            )
            .expect("insert");
        connection
            .execute("DELETE FROM retrieval_documents WHERE id = 'agent_memory:m1'", [])
            .expect("delete");

        let hits: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM retrieval_documents_fts WHERE retrieval_documents_fts MATCH '\"npm\"'",
                [],
                |row| row.get(0),
            )
            .expect("fts count");
        assert_eq!(hits, 0);
    }

    #[test]
    fn the_same_source_cannot_be_indexed_twice() {
        let connection = migrated_connection();
        let insert = "INSERT INTO retrieval_documents
             (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash, created_at, updated_at)
             VALUES ('agent_memory:m1','agent_memory','m1','a','', 'x', 'h', 't', 't')";
        connection.execute(insert, []).expect("first insert");
        assert!(connection.execute(insert, []).is_err());
    }

    #[test]
    fn the_configuration_table_holds_at_most_one_row() {
        let connection = migrated_connection();
        connection
            .execute(
                "INSERT INTO retrieval_configuration (id, source_profile_id, embedding_model, updated_at)
                 VALUES (1, 'p1', 'text-embedding-3-small', 't')",
                [],
            )
            .expect("singleton insert");
        assert!(connection
            .execute(
                "INSERT INTO retrieval_configuration (id, source_profile_id, embedding_model, updated_at)
                 VALUES (2, 'p2', 'other', 't')",
                [],
            )
            .is_err());
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::infrastructure::schema`
Expected: FAIL —— `cannot find function apply_retrieval_schema`。

- [ ] **Step 3: 写最小实现**

`src-tauri/src/contexts/retrieval/infrastructure/schema.rs`：

```rust
use crate::platform::database::DatabaseError;
use rusqlite::Connection;

/// 迁移 42 `retrieval-vector-index`（`add-retrieval-vector-search`）。
///
/// scope 冗余进本表而不是每次 JOIN 回源表：检索先按 `scope_agent_id + scope_folder` 过滤再
/// 暴力扫描候选集，这是"不建 ANN 索引也够快"成立的前提。
/// FTS 建在本表而非 `agent_memories`：第 2/3 期的源表不同，统一在本表做 FTS 才能让混合检索
/// 只实现一次。
/// 不建到 `agent_memories` 的外键：跨期源表不同，靠 `source_kind + source_id` 逻辑关联，
/// 检索结果一律回查源表，源已删则跳过。
pub(crate) fn apply_retrieval_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS retrieval_documents (
            id TEXT PRIMARY KEY,
            source_kind TEXT NOT NULL,
            source_id TEXT NOT NULL,
            scope_agent_id TEXT NOT NULL,
            scope_folder TEXT NOT NULL DEFAULT '',
            content TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            index_state TEXT NOT NULL DEFAULT 'pending',
            attempt_count INTEGER NOT NULL DEFAULT 0,
            failure_category TEXT,
            embedding_model TEXT,
            embedding_dimensions INTEGER,
            embedding BLOB,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE (source_kind, source_id)
        );

        CREATE INDEX IF NOT EXISTS idx_retrieval_documents_scope
            ON retrieval_documents (source_kind, scope_agent_id, scope_folder, index_state);
        CREATE INDEX IF NOT EXISTS idx_retrieval_documents_queue
            ON retrieval_documents (index_state, updated_at);

        CREATE VIRTUAL TABLE IF NOT EXISTS retrieval_documents_fts USING fts5(
            content,
            content='retrieval_documents',
            content_rowid='rowid',
            tokenize='trigram'
        );

        CREATE TRIGGER IF NOT EXISTS retrieval_documents_fts_insert
        AFTER INSERT ON retrieval_documents BEGIN
            INSERT INTO retrieval_documents_fts(rowid, content) VALUES (new.rowid, new.content);
        END;
        CREATE TRIGGER IF NOT EXISTS retrieval_documents_fts_delete
        AFTER DELETE ON retrieval_documents BEGIN
            INSERT INTO retrieval_documents_fts(retrieval_documents_fts, rowid, content)
            VALUES ('delete', old.rowid, old.content);
        END;
        CREATE TRIGGER IF NOT EXISTS retrieval_documents_fts_update
        AFTER UPDATE OF content ON retrieval_documents BEGIN
            INSERT INTO retrieval_documents_fts(retrieval_documents_fts, rowid, content)
            VALUES ('delete', old.rowid, old.content);
            INSERT INTO retrieval_documents_fts(rowid, content) VALUES (new.rowid, new.content);
        END;

        -- 单例配置行。retrieval 拥有自己的配置表，而不是借用 desktop 上下文的 settings KV 表，
        -- 避免为读一条自有配置去依赖另一个上下文的 api。
        CREATE TABLE IF NOT EXISTS retrieval_configuration (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            source_profile_id TEXT,
            embedding_model TEXT,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}
```

`src-tauri/src/contexts/retrieval/infrastructure/mod.rs`：

```rust
pub(crate) mod schema;

pub(crate) use schema::apply_retrieval_schema;
```

`src-tauri/src/contexts/retrieval/mod.rs` 追加一行 `pub(crate) mod infrastructure;`。

`migrations.rs` 在 41 的 `apply_migration(...)` 之后、`Ok(())` 之前插入：

```rust
    apply_migration(
        conn,
        42,
        "retrieval-vector-index",
        crate::contexts::retrieval::infrastructure::apply_retrieval_schema,
    )?;
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::infrastructure::schema`
Expected: PASS，6 个测试。

- [ ] **Step 5: 补迁移回归测试**

在 `src-tauri/src/migration_fixture_tests.rs` 追加（照该文件既有用例的风格）：

```rust
#[test]
fn upgrading_an_existing_database_to_the_retrieval_index_preserves_data() {
    let directory = TempDirectory::new("retrieval migration upgrade");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("first open");
    {
        let connection = database.connection().expect("migrated");
        connection
            .execute(
                "INSERT INTO agents (id, display_name, provider, launch_kind)
                 VALUES ('a', 'A', 'Test', 'api')",
                [],
            )
            .expect("seed agent");
    }
    drop(database);

    let reopened = NativeDatabase::new(directory.path().to_path_buf()).expect("reopen");
    let connection = reopened.connection().expect("migrated again");
    let agents: i64 = connection
        .query_row("SELECT COUNT(*) FROM agents", [], |row| row.get(0))
        .expect("agent count");
    let documents: i64 = connection
        .query_row("SELECT COUNT(*) FROM retrieval_documents", [], |row| row.get(0))
        .expect("document count");

    assert_eq!(agents, 1, "existing rows must survive migration 42");
    assert_eq!(documents, 0);
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml migration_fixture_tests`
Expected: PASS。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/contexts/retrieval src-tauri/src/platform/database/migrations.rs src-tauri/src/migration_fixture_tests.rs
git commit -m "feat(retrieval): add migration 42 with FTS5 shadow index"
```

---

### Task 5: 文档仓储

**Files:**
- Create: `src-tauri/src/contexts/retrieval/infrastructure/sqlite_repository.rs`
- Create: `src-tauri/src/contexts/retrieval/application/mod.rs`
- Create: `src-tauri/src/contexts/retrieval/application/ports.rs`
- Modify: `src-tauri/src/contexts/retrieval/infrastructure/mod.rs`、`src-tauri/src/contexts/retrieval/mod.rs`
- Modify: `src-tauri/src/contexts/retrieval/domain/document.rs`（本任务的 trait 签名用上了 `RetrievalDocument` 与 `FailureCategory`，删掉 Task 2 临时加的 `#[allow(dead_code)]`）

**Interfaces:**
- Consumes: Task 2 的 `SourceKind/IndexState/FailureCategory/RetrievalDocument/document_id`；Task 3 的 `encode_embedding/decode_embedding/escape_fts_query`
- Produces: trait `RetrievalDocumentRepository` 与实现 `SqliteRetrievalDocumentRepository`，方法签名如下（Task 7/8/9 依赖）：

```rust
pub(crate) trait RetrievalDocumentRepository: Send + Sync {
    fn upsert_pending(&self, document: &RetrievalDocument) -> Result<(), RetrievalError>;
    fn list_indexed_source_ids(&self, source_kind: SourceKind) -> Result<Vec<(String, String)>, RetrievalError>;
    fn delete_by_source(&self, source_kind: SourceKind, source_id: &str) -> Result<(), RetrievalError>;
    fn claim_pending_batch(&self, source_kind: SourceKind, limit: usize) -> Result<Vec<RetrievalDocument>, RetrievalError>;
    fn store_embedding(&self, id: &str, model: &str, embedding: &[f32]) -> Result<(), RetrievalError>;
    fn record_failure(&self, id: &str, category: FailureCategory, give_up: bool) -> Result<(), RetrievalError>;
    fn vector_candidates(&self, scope: &RetrievalScope, source_kind: SourceKind, model: &str) -> Result<Vec<(String, Vec<f32>)>, RetrievalError>;
    fn keyword_candidates(&self, scope: &RetrievalScope, source_kind: SourceKind, query: &str, limit: usize) -> Result<Vec<String>, RetrievalError>;
    fn index_status(&self, agent_id: &str) -> Result<RetrievalIndexStatus, RetrievalError>;
    fn requeue_all(&self, agent_id: &str) -> Result<(), RetrievalError>;
}
```

`list_indexed_source_ids` 返回 `(source_id, content_hash)`——reconcile 只需要这两列就能算出全部三类待办。
`vector_candidates` 与 `keyword_candidates` 返回的是 `source_id`（不是行 id），因为消费方要拿它回查源表。

- [ ] **Step 1: 写失败的测试**

`sqlite_repository.rs` 末尾。fixture 照搬 `memory_repository.rs:170-200` 的 `TempDirectory` + `NativeDatabase`：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    struct Fixture {
        _directory: TempDirectory,
        repository: SqliteRetrievalDocumentRepository,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let directory = TempDirectory::new(label);
            let database =
                NativeDatabase::new(directory.path().to_path_buf()).expect("test database");
            Self {
                repository: SqliteRetrievalDocumentRepository::new(database),
                _directory: directory,
            }
        }
    }

    fn document(source_id: &str, agent: &str, folder: &str, content: &str) -> RetrievalDocument {
        RetrievalDocument {
            id: document_id(SourceKind::AgentMemory, source_id),
            source_kind: SourceKind::AgentMemory,
            source_id: source_id.to_string(),
            scope_agent_id: agent.to_string(),
            scope_folder: folder.to_string(),
            content: content.to_string(),
            content_hash: content_hash(content),
            index_state: IndexState::Pending,
            attempt_count: 0,
            embedding_model: None,
        }
    }

    fn scope(agent: &str, folder: &str) -> RetrievalScope {
        RetrievalScope { agent_id: agent.to_string(), folder: folder.to_string() }
    }

    #[test]
    fn upsert_is_idempotent_and_refreshes_content_and_hash() {
        let fixture = Fixture::new("retrieval upsert idempotent");
        fixture.repository.upsert_pending(&document("m1", "a", "", "uses npm")).expect("first");
        fixture.repository.upsert_pending(&document("m1", "a", "", "uses cargo")).expect("second");

        let indexed = fixture
            .repository
            .list_indexed_source_ids(SourceKind::AgentMemory)
            .expect("list");
        assert_eq!(indexed.len(), 1);
        assert_eq!(indexed[0].0, "m1");
        assert_eq!(indexed[0].1, content_hash("uses cargo"));
    }

    #[test]
    fn storing_an_embedding_marks_the_row_indexed_and_clears_failure_state() {
        let fixture = Fixture::new("retrieval store embedding");
        fixture.repository.upsert_pending(&document("m1", "a", "", "uses npm")).expect("upsert");
        fixture
            .repository
            .record_failure(&document_id(SourceKind::AgentMemory, "m1"), FailureCategory::Network, false)
            .expect("failure");

        fixture
            .repository
            .store_embedding(&document_id(SourceKind::AgentMemory, "m1"), "model-a", &[1.0, 0.0])
            .expect("store");

        let candidates = fixture
            .repository
            .vector_candidates(&scope("a", ""), SourceKind::AgentMemory, "model-a")
            .expect("candidates");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].0, "m1");
        assert_eq!(candidates[0].1, vec![1.0, 0.0]);
    }

    #[test]
    fn vector_candidates_exclude_rows_embedded_with_a_different_model() {
        let fixture = Fixture::new("retrieval model mismatch");
        fixture.repository.upsert_pending(&document("m1", "a", "", "x")).expect("upsert");
        fixture
            .repository
            .store_embedding(&document_id(SourceKind::AgentMemory, "m1"), "old-model", &[1.0, 0.0])
            .expect("store");

        let candidates = fixture
            .repository
            .vector_candidates(&scope("a", ""), SourceKind::AgentMemory, "new-model")
            .expect("candidates");
        assert!(candidates.is_empty());
    }

    #[test]
    fn candidates_never_cross_agent_or_folder_boundaries() {
        let fixture = Fixture::new("retrieval scope isolation");
        for (source_id, agent, folder) in [
            ("m1", "a", "D:/one"),
            ("m2", "a", "D:/two"),
            ("m3", "b", "D:/one"),
        ] {
            fixture
                .repository
                .upsert_pending(&document(source_id, agent, folder, "shared content"))
                .expect("upsert");
            fixture
                .repository
                .store_embedding(&document_id(SourceKind::AgentMemory, source_id), "m", &[1.0, 0.0])
                .expect("store");
        }

        let vectors = fixture
            .repository
            .vector_candidates(&scope("a", "D:/one"), SourceKind::AgentMemory, "m")
            .expect("vectors");
        let keywords = fixture
            .repository
            .keyword_candidates(&scope("a", "D:/one"), SourceKind::AgentMemory, "\"shared\"", 10)
            .expect("keywords");

        assert_eq!(vectors.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(), vec!["m1"]);
        assert_eq!(keywords, vec!["m1".to_string()]);
    }

    #[test]
    fn keyword_candidates_find_pending_rows_because_fts_does_not_wait_for_the_worker() {
        let fixture = Fixture::new("retrieval keyword pending");
        fixture.repository.upsert_pending(&document("m1", "a", "", "uses npm not pnpm")).expect("upsert");

        let hits = fixture
            .repository
            .keyword_candidates(&scope("a", ""), SourceKind::AgentMemory, "\"pnpm\"", 10)
            .expect("keywords");
        assert_eq!(hits, vec!["m1".to_string()]);
    }

    #[test]
    fn claim_pending_batch_respects_its_limit_and_skips_indexed_rows() {
        let fixture = Fixture::new("retrieval claim batch");
        for source_id in ["m1", "m2", "m3"] {
            fixture
                .repository
                .upsert_pending(&document(source_id, "a", "", "content"))
                .expect("upsert");
        }
        fixture
            .repository
            .store_embedding(&document_id(SourceKind::AgentMemory, "m1"), "m", &[1.0])
            .expect("store");

        let batch = fixture
            .repository
            .claim_pending_batch(SourceKind::AgentMemory, 2)
            .expect("batch");
        assert_eq!(batch.len(), 2);
        assert!(batch.iter().all(|document| document.source_id != "m1"));
    }

    #[test]
    fn giving_up_marks_failed_while_a_retryable_failure_stays_pending() {
        let fixture = Fixture::new("retrieval failure states");
        fixture.repository.upsert_pending(&document("m1", "a", "", "x")).expect("m1");
        fixture.repository.upsert_pending(&document("m2", "a", "", "x")).expect("m2");

        fixture
            .repository
            .record_failure(&document_id(SourceKind::AgentMemory, "m1"), FailureCategory::Auth, true)
            .expect("give up");
        fixture
            .repository
            .record_failure(&document_id(SourceKind::AgentMemory, "m2"), FailureCategory::Network, false)
            .expect("retry later");

        let status = fixture.repository.index_status("a").expect("status");
        assert_eq!(status.failed, 1);
        assert_eq!(status.pending, 1);
        assert_eq!(status.indexed, 0);
        assert_eq!(status.last_failure_category.as_deref(), Some("auth"));
    }

    #[test]
    fn requeue_all_resets_failures_and_attempt_counts() {
        let fixture = Fixture::new("retrieval requeue");
        fixture.repository.upsert_pending(&document("m1", "a", "", "x")).expect("upsert");
        fixture
            .repository
            .record_failure(&document_id(SourceKind::AgentMemory, "m1"), FailureCategory::Auth, true)
            .expect("failure");

        fixture.repository.requeue_all("a").expect("requeue");

        let status = fixture.repository.index_status("a").expect("status");
        assert_eq!(status.failed, 0);
        assert_eq!(status.pending, 1);
        assert_eq!(status.last_failure_category, None);
    }

    #[test]
    fn index_status_spans_every_folder_of_the_agent() {
        let fixture = Fixture::new("retrieval status folders");
        fixture.repository.upsert_pending(&document("m1", "a", "D:/one", "x")).expect("m1");
        fixture.repository.upsert_pending(&document("m2", "a", "D:/two", "x")).expect("m2");
        fixture.repository.upsert_pending(&document("m3", "b", "D:/one", "x")).expect("m3");

        let status = fixture.repository.index_status("a").expect("status");
        assert_eq!(status.pending, 2);
    }

    #[test]
    fn delete_by_source_removes_the_row_and_its_fts_entry() {
        let fixture = Fixture::new("retrieval delete");
        fixture.repository.upsert_pending(&document("m1", "a", "", "uses npm")).expect("upsert");

        fixture
            .repository
            .delete_by_source(SourceKind::AgentMemory, "m1")
            .expect("delete");

        let keywords = fixture
            .repository
            .keyword_candidates(&scope("a", ""), SourceKind::AgentMemory, "\"npm\"", 10)
            .expect("keywords");
        assert!(keywords.is_empty());
        assert!(fixture
            .repository
            .list_indexed_source_ids(SourceKind::AgentMemory)
            .expect("list")
            .is_empty());
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::infrastructure::sqlite_repository`
Expected: FAIL —— `cannot find type SqliteRetrievalDocumentRepository`。

- [ ] **Step 3: 写 ports 与实现**

`src-tauri/src/contexts/retrieval/application/ports.rs` 先只放本任务需要的两项：

```rust
use crate::contexts::retrieval::domain::{
    FailureCategory, RetrievalDocument, RetrievalError, RetrievalScope, SourceKind,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RetrievalIndexStatus {
    pub(crate) indexed: u32,
    pub(crate) pending: u32,
    pub(crate) failed: u32,
    /// 只给类别，不带原始错误文本——错误体可能含凭据或 provider 响应内容（设计文档 §8.2）。
    pub(crate) last_failure_category: Option<String>,
}

pub(crate) trait RetrievalDocumentRepository: Send + Sync {
    fn upsert_pending(&self, document: &RetrievalDocument) -> Result<(), RetrievalError>;
    fn list_indexed_source_ids(
        &self,
        source_kind: SourceKind,
    ) -> Result<Vec<(String, String)>, RetrievalError>;
    fn delete_by_source(
        &self,
        source_kind: SourceKind,
        source_id: &str,
    ) -> Result<(), RetrievalError>;
    fn claim_pending_batch(
        &self,
        source_kind: SourceKind,
        limit: usize,
    ) -> Result<Vec<RetrievalDocument>, RetrievalError>;
    fn store_embedding(
        &self,
        id: &str,
        model: &str,
        embedding: &[f32],
    ) -> Result<(), RetrievalError>;
    fn record_failure(
        &self,
        id: &str,
        category: FailureCategory,
        give_up: bool,
    ) -> Result<(), RetrievalError>;
    fn vector_candidates(
        &self,
        scope: &RetrievalScope,
        source_kind: SourceKind,
        model: &str,
    ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError>;
    fn keyword_candidates(
        &self,
        scope: &RetrievalScope,
        source_kind: SourceKind,
        query: &str,
        limit: usize,
    ) -> Result<Vec<String>, RetrievalError>;
    fn index_status(&self, agent_id: &str) -> Result<RetrievalIndexStatus, RetrievalError>;
    fn requeue_all(&self, agent_id: &str) -> Result<(), RetrievalError>;
}
```

`application/mod.rs`：

```rust
pub(crate) mod ports;

pub(crate) use ports::{RetrievalDocumentRepository, RetrievalIndexStatus};
```

`sqlite_repository.rs` 实现要点（完整实现按下列 SQL 逐个方法填；此处给出全部关键语句）：

```rust
use crate::contexts::retrieval::application::{RetrievalDocumentRepository, RetrievalIndexStatus};
use crate::contexts::retrieval::domain::{
    content_hash, decode_embedding, document_id, encode_embedding, FailureCategory, IndexState,
    RetrievalDocument, RetrievalError, RetrievalScope, SourceKind,
};
use crate::platform::clock::SystemClock;
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Row};

#[derive(Clone)]
pub(crate) struct SqliteRetrievalDocumentRepository {
    database: NativeDatabase,
}

impl SqliteRetrievalDocumentRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}
```

各方法的 SQL：

```sql
-- upsert_pending：内容变了才重置索引状态，否则重复 reconcile 会把已索引行反复打回 pending。
INSERT INTO retrieval_documents
    (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash,
     index_state, attempt_count, failure_category, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', 0, NULL, ?8, ?8)
ON CONFLICT(id) DO UPDATE SET
    content = excluded.content,
    content_hash = excluded.content_hash,
    scope_agent_id = excluded.scope_agent_id,
    scope_folder = excluded.scope_folder,
    index_state = CASE WHEN retrieval_documents.content_hash = excluded.content_hash
                       THEN retrieval_documents.index_state ELSE 'pending' END,
    attempt_count = CASE WHEN retrieval_documents.content_hash = excluded.content_hash
                         THEN retrieval_documents.attempt_count ELSE 0 END,
    failure_category = CASE WHEN retrieval_documents.content_hash = excluded.content_hash
                            THEN retrieval_documents.failure_category ELSE NULL END,
    updated_at = excluded.updated_at;

-- list_indexed_source_ids
SELECT source_id, content_hash FROM retrieval_documents WHERE source_kind = ?1;

-- delete_by_source
DELETE FROM retrieval_documents WHERE source_kind = ?1 AND source_id = ?2;

-- claim_pending_batch
SELECT id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash,
       index_state, attempt_count, embedding_model
FROM retrieval_documents
WHERE source_kind = ?1 AND index_state = 'pending'
ORDER BY updated_at ASC
LIMIT ?2;

-- store_embedding
UPDATE retrieval_documents
SET embedding = ?2, embedding_model = ?3, embedding_dimensions = ?4,
    index_state = 'indexed', failure_category = NULL, attempt_count = 0, updated_at = ?5
WHERE id = ?1;

-- record_failure：give_up 决定终态，attempt_count 由本语句自增，调用方不需要先读。
UPDATE retrieval_documents
SET attempt_count = attempt_count + 1,
    failure_category = ?2,
    index_state = CASE WHEN ?3 THEN 'failed' ELSE 'pending' END,
    updated_at = ?4
WHERE id = ?1;

-- vector_candidates：刻意不取 content，只拉 BLOB；内容统一由消费方回查源表提供。
SELECT source_id, embedding FROM retrieval_documents
WHERE source_kind = ?1 AND scope_agent_id = ?2 AND scope_folder = ?3
  AND index_state = 'indexed' AND embedding_model = ?4 AND embedding IS NOT NULL;

-- keyword_candidates
SELECT d.source_id FROM retrieval_documents d
JOIN retrieval_documents_fts f ON f.rowid = d.rowid
WHERE retrieval_documents_fts MATCH ?1
  AND d.source_kind = ?2 AND d.scope_agent_id = ?3 AND d.scope_folder = ?4
ORDER BY bm25(retrieval_documents_fts)
LIMIT ?5;

-- index_status
SELECT
  SUM(index_state = 'indexed'), SUM(index_state = 'pending'), SUM(index_state = 'failed'),
  (SELECT failure_category FROM retrieval_documents
   WHERE scope_agent_id = ?1 AND failure_category IS NOT NULL
   ORDER BY updated_at DESC LIMIT 1)
FROM retrieval_documents WHERE scope_agent_id = ?1;

-- requeue_all
UPDATE retrieval_documents
SET index_state = 'pending', attempt_count = 0, failure_category = NULL, updated_at = ?2
WHERE scope_agent_id = ?1;
```

时间戳一律用 `SystemClock.rfc3339()`。行读取写一个 `DocumentRow::read(&Row) -> rusqlite::Result<Self>` + `into_document()`，照 `memory_repository.rs:115-152` 的 `MemoryRow` 写法；`SourceKind`/`IndexState` 解析失败时返回 `RetrievalError::Storage`。rusqlite 错误统一经一个 `fn storage_error(error: rusqlite::Error) -> RetrievalError` 转换。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::infrastructure::sqlite_repository`
Expected: PASS，10 个测试。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/contexts/retrieval
git commit -m "feat(retrieval): add SQLite document repository with scope-filtered recall"
```

---

### Task 6: 配置仓储

**Files:**
- Create: `src-tauri/src/contexts/retrieval/infrastructure/configuration_repository.rs`
- Modify: `src-tauri/src/contexts/retrieval/application/ports.rs`、`application/mod.rs`、`infrastructure/mod.rs`

**Interfaces:**
- Produces:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RetrievalConfiguration {
    pub(crate) source_profile_id: Option<String>,
    pub(crate) embedding_model: Option<String>,
}

impl RetrievalConfiguration {
    /// 两者齐备才算"已配置"——缺任一个都无法发起一次 embedding 调用。
    pub(crate) fn resolved_model(&self) -> Option<(&str, &str)>;
}

pub(crate) trait RetrievalConfigurationRepository: Send + Sync {
    fn load(&self) -> Result<RetrievalConfiguration, RetrievalError>;
    fn save(&self, profile_id: &str, embedding_model: &str) -> Result<(), RetrievalError>;
}
```

- [ ] **Step 1: 写失败的测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn fixture(label: &str) -> (TempDirectory, SqliteRetrievalConfigurationRepository) {
        let directory = TempDirectory::new(label);
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        (directory, SqliteRetrievalConfigurationRepository::new(database))
    }

    #[test]
    fn an_unconfigured_database_loads_an_empty_configuration() {
        let (_directory, repository) = fixture("retrieval config empty");
        let configuration = repository.load().expect("load");
        assert_eq!(configuration, RetrievalConfiguration::default());
        assert_eq!(configuration.resolved_model(), None);
    }

    #[test]
    fn saving_twice_updates_the_single_row_instead_of_failing() {
        let (_directory, repository) = fixture("retrieval config overwrite");
        repository.save("profile-a", "model-a").expect("first save");
        repository.save("profile-b", "model-b").expect("second save");

        let configuration = repository.load().expect("load");
        assert_eq!(configuration.resolved_model(), Some(("profile-b", "model-b")));
    }

    #[test]
    fn a_configuration_missing_either_half_is_not_resolved() {
        assert_eq!(
            RetrievalConfiguration {
                source_profile_id: Some("p".to_string()),
                embedding_model: None,
            }
            .resolved_model(),
            None
        );
        assert_eq!(
            RetrievalConfiguration {
                source_profile_id: None,
                embedding_model: Some("m".to_string()),
            }
            .resolved_model(),
            None
        );
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::infrastructure::configuration_repository`
Expected: FAIL。

- [ ] **Step 3: 写实现**

`resolved_model`：

```rust
impl RetrievalConfiguration {
    pub(crate) fn resolved_model(&self) -> Option<(&str, &str)> {
        let profile = self.source_profile_id.as_deref()?;
        let model = self.embedding_model.as_deref()?;
        (!profile.is_empty() && !model.is_empty()).then_some((profile, model))
    }
}
```

仓储 SQL：

```sql
-- load
SELECT source_profile_id, embedding_model FROM retrieval_configuration WHERE id = 1;

-- save
INSERT INTO retrieval_configuration (id, source_profile_id, embedding_model, updated_at)
VALUES (1, ?1, ?2, ?3)
ON CONFLICT(id) DO UPDATE SET
    source_profile_id = excluded.source_profile_id,
    embedding_model = excluded.embedding_model,
    updated_at = excluded.updated_at;
```

`load` 在无行时返回 `RetrievalConfiguration::default()`，不是错误。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::infrastructure::configuration_repository`
Expected: PASS，3 个测试。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/contexts/retrieval
git commit -m "feat(retrieval): add singleton retrieval configuration repository"
```

---

### Task 7: 索引服务的差集协调

**Files:**
- Create: `src-tauri/src/contexts/retrieval/application/indexing_service.rs`
- Modify: `src-tauri/src/contexts/retrieval/application/ports.rs`、`application/mod.rs`

**Interfaces:**
- Consumes: Task 5 的 `RetrievalDocumentRepository`
- Produces:

```rust
/// retrieval 从源上下文取快照的消费侧契约。第 1 期唯一实现是 agent_runtime 的记忆表适配器。
pub(crate) trait IndexSourcePort: Send + Sync {
    fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexSourceRecord {
    pub(crate) source_id: String,
    pub(crate) agent_id: String,
    pub(crate) folder: String,
    pub(crate) content: String,
    /// 检索结果要带上它（Task 9 的 `ScoredHit.created_at`），且它只存在于源表——
    /// 索引行刻意不复制这个字段，避免又多一处会陈旧的副本。
    pub(crate) created_at: String,
}

pub(crate) struct IndexingService { /* repository + source + logging */ }

impl IndexingService {
    pub(crate) fn reconcile(&self) -> Result<ReconcileOutcome, RetrievalError>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReconcileOutcome {
    pub(crate) added: usize,
    pub(crate) invalidated: usize,
    pub(crate) orphans_removed: usize,
}
```

- [ ] **Step 1: 写失败的测试**

用内存 fake，不碰数据库。**这套 fake 是 Task 8/9 的模板，后续任务按需给它补方法即可。**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakeSource {
        records: Vec<IndexSourceRecord>,
    }

    impl IndexSourcePort for FakeSource {
        fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
            Ok(self.records.clone())
        }
    }

    #[derive(Default)]
    struct FakeRepository {
        /// 已存在的索引行：(source_id, content_hash)
        rows: Vec<(String, String)>,
        upserted: Mutex<Vec<String>>,
        deleted: Mutex<Vec<String>>,
    }

    impl RetrievalDocumentRepository for FakeRepository {
        fn upsert_pending(&self, document: &RetrievalDocument) -> Result<(), RetrievalError> {
            self.upserted.lock().expect("lock").push(document.source_id.clone());
            Ok(())
        }

        fn list_indexed_source_ids(
            &self,
            _source_kind: SourceKind,
        ) -> Result<Vec<(String, String)>, RetrievalError> {
            Ok(self.rows.clone())
        }

        fn delete_by_source(
            &self,
            _source_kind: SourceKind,
            source_id: &str,
        ) -> Result<(), RetrievalError> {
            self.deleted.lock().expect("lock").push(source_id.to_string());
            Ok(())
        }

        // reconcile 只用到上面三个方法。其余方法在本套测试中不可达，走 unimplemented!()。
        fn claim_pending_batch(
            &self,
            _source_kind: SourceKind,
            _limit: usize,
        ) -> Result<Vec<RetrievalDocument>, RetrievalError> {
            unimplemented!("not exercised by reconcile tests")
        }
        fn store_embedding(
            &self,
            _id: &str,
            _model: &str,
            _embedding: &[f32],
        ) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by reconcile tests")
        }
        fn record_failure(
            &self,
            _id: &str,
            _category: FailureCategory,
            _give_up: bool,
        ) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by reconcile tests")
        }
        fn vector_candidates(
            &self,
            _scope: &RetrievalScope,
            _source_kind: SourceKind,
            _model: &str,
        ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError> {
            unimplemented!("not exercised by reconcile tests")
        }
        fn keyword_candidates(
            &self,
            _scope: &RetrievalScope,
            _source_kind: SourceKind,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<String>, RetrievalError> {
            unimplemented!("not exercised by reconcile tests")
        }
        fn index_status(&self, _agent_id: &str) -> Result<RetrievalIndexStatus, RetrievalError> {
            unimplemented!("not exercised by reconcile tests")
        }
        fn requeue_all(&self, _agent_id: &str) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by reconcile tests")
        }
    }

    fn record(source_id: &str, content: &str) -> IndexSourceRecord {
        IndexSourceRecord {
            source_id: source_id.to_string(),
            agent_id: "a".to_string(),
            folder: String::new(),
            content: content.to_string(),
            created_at: "2026-08-05T00:00:00Z".to_string(),
        }
    }

    fn indexed(source_id: &str, content: &str) -> (String, String) {
        (source_id.to_string(), content_hash(content))
    }

    /// 装配一个只依赖两个 fake 的服务，返回服务与仓储句柄以便事后断言调用记录。
    fn service(
        records: Vec<IndexSourceRecord>,
        rows: Vec<(String, String)>,
    ) -> (IndexingService, Arc<FakeRepository>) {
        let repository = Arc::new(FakeRepository { rows, ..FakeRepository::default() });
        let service = IndexingService::new(
            repository.clone(),
            Arc::new(FakeSource { records }),
        );
        (service, repository)
    }

    #[test]
    fn a_source_record_with_no_index_row_is_added() {
        let (service, repository) = service(vec![record("m1", "uses npm")], Vec::new());

        let outcome = service.reconcile().expect("reconcile");

        assert_eq!(outcome.added, 1);
        assert_eq!(outcome.invalidated, 0);
        assert_eq!(outcome.orphans_removed, 0);
        assert_eq!(*repository.upserted.lock().expect("lock"), vec!["m1".to_string()]);
    }

    #[test]
    fn a_content_change_invalidates_the_existing_index_row() {
        let (service, repository) = service(
            vec![record("m1", "uses cargo")],
            vec![indexed("m1", "uses npm")],
        );

        let outcome = service.reconcile().expect("reconcile");

        assert_eq!(outcome.invalidated, 1);
        assert_eq!(outcome.added, 0);
        assert_eq!(*repository.upserted.lock().expect("lock"), vec!["m1".to_string()]);
    }

    #[test]
    fn an_index_row_whose_source_disappeared_is_removed() {
        let (service, repository) = service(Vec::new(), vec![indexed("m1", "uses npm")]);

        let outcome = service.reconcile().expect("reconcile");

        assert_eq!(outcome.orphans_removed, 1);
        assert_eq!(*repository.deleted.lock().expect("lock"), vec!["m1".to_string()]);
    }

    #[test]
    fn an_unchanged_record_is_left_alone() {
        let (service, repository) = service(
            vec![record("m1", "uses npm")],
            vec![indexed("m1", "uses npm")],
        );

        let outcome = service.reconcile().expect("reconcile");

        assert_eq!(outcome, ReconcileOutcome::default());
        assert!(repository.upserted.lock().expect("lock").is_empty());
        assert!(repository.deleted.lock().expect("lock").is_empty());
    }

    #[test]
    fn all_three_kinds_of_work_are_handled_in_one_pass() {
        let (service, repository) = service(
            vec![record("m1", "new"), record("m2", "changed")],
            vec![indexed("m2", "original"), indexed("m3", "orphan")],
        );

        let outcome = service.reconcile().expect("reconcile");

        assert_eq!(outcome.added, 1);
        assert_eq!(outcome.invalidated, 1);
        assert_eq!(outcome.orphans_removed, 1);
        assert_eq!(*repository.deleted.lock().expect("lock"), vec!["m3".to_string()]);
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::application::indexing_service`
Expected: FAIL。

- [ ] **Step 3: 写实现**

```rust
/// 索引的真源是这一次差集协调，而**不是**保存路径上的双写。
///
/// 在 `SqliteAgentMemoryRepository::save` 里顺手插一条索引行看似更简单，但那会引入
/// "入队写失败 → 该记忆永远搜不到"的静默漏洞：保存成功了，用户以为记住了，检索却永远看不见。
/// 协调式的代价只是最多延迟一个周期，而且顺带把历史存量记忆回填掉，不需要单独的数据迁移脚本。
pub(crate) fn reconcile(&self) -> Result<ReconcileOutcome, RetrievalError> {
    let records = self.source.snapshot()?;
    let existing: HashMap<String, String> = self
        .repository
        .list_indexed_source_ids(SourceKind::AgentMemory)?
        .into_iter()
        .collect();

    let mut outcome = ReconcileOutcome::default();
    let mut live: HashSet<&str> = HashSet::new();
    for record in &records {
        live.insert(record.source_id.as_str());
        let hash = content_hash(&record.content);
        match existing.get(&record.source_id) {
            Some(existing_hash) if existing_hash == &hash => continue,
            Some(_) => outcome.invalidated += 1,
            None => outcome.added += 1,
        }
        self.repository.upsert_pending(&RetrievalDocument {
            id: document_id(SourceKind::AgentMemory, &record.source_id),
            source_kind: SourceKind::AgentMemory,
            source_id: record.source_id.clone(),
            scope_agent_id: record.agent_id.clone(),
            scope_folder: record.folder.clone(),
            content: record.content.clone(),
            content_hash: hash,
            index_state: IndexState::Pending,
            attempt_count: 0,
            embedding_model: None,
        })?;
    }

    // 孤儿清理是 §5.3 显式撤销失败时的兜底。少了它，一次失败的撤销调用会让索引行永久残留。
    for source_id in existing.keys() {
        if !live.contains(source_id.as_str()) {
            self.repository
                .delete_by_source(SourceKind::AgentMemory, source_id)?;
            outcome.orphans_removed += 1;
        }
    }
    Ok(outcome)
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::application::indexing_service`
Expected: PASS，5 个测试。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/contexts/retrieval
git commit -m "feat(retrieval): reconcile index against its source instead of dual-writing"
```

---

### Task 8: 索引 worker 的批处理、失败分类与重试

**Files:**
- Modify: `src-tauri/src/contexts/retrieval/application/indexing_service.rs`
- Modify: `src-tauri/src/contexts/retrieval/application/ports.rs`（加 `EmbeddingPort`）

**Interfaces:**
- Produces:

```rust
/// 可调常量集中在此，不散落调用点（设计文档 §5.2）。
pub(crate) const EMBEDDING_BATCH_SIZE: usize = 32;
pub(crate) const RECONCILE_POLL_INTERVAL_SECONDS: u64 = 300;
pub(crate) const MAX_EMBEDDING_ATTEMPTS: u32 = 5;
pub(crate) const RETRY_BACKOFF_SECONDS: [u64; 5] = [1, 4, 15, 60, 300];
/// 超长内容 embedding 前截断；FTS 仍索引全文，所以长记忆的尾部仍可被关键词命中。
pub(crate) const EMBEDDING_CONTENT_LIMIT: usize = 8000;

pub(crate) trait EmbeddingPort: Send + Sync {
    fn embed(&self, model: &str, inputs: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingFailure>;
}

pub(crate) struct EmbeddingFailure {
    pub(crate) category: FailureCategory,
    pub(crate) message: String,
}

impl IndexingService {
    pub(crate) fn process_pending_batch(&self, model: &str) -> Result<BatchOutcome, RetrievalError>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BatchOutcome {
    pub(crate) succeeded: usize,
    pub(crate) failed: usize,
}
```

- [ ] **Step 1: 写失败的测试**

```rust
#[test]
fn a_successful_batch_stores_one_embedding_per_document() { /* fake 返回 N 个向量 → succeeded = N */ }

#[test]
fn an_auth_failure_gives_up_immediately_without_burning_quota() {
    // fake 返回 FailureCategory::Auth → record_failure(give_up = true)，即使 attempt_count 是 0
}

#[test]
fn an_invalid_request_failure_gives_up_immediately() { /* 同上，category = InvalidRequest */ }

#[test]
fn a_network_failure_below_the_attempt_ceiling_stays_retryable() {
    // attempt_count = 1 的行 + Network → record_failure(give_up = false)
}

#[test]
fn a_network_failure_at_the_attempt_ceiling_gives_up() {
    // attempt_count = MAX_EMBEDDING_ATTEMPTS - 1 → record_failure(give_up = true)
}

#[test]
fn content_longer_than_the_limit_is_truncated_before_embedding() {
    // 输入 8001 字符 → fake 收到的字符串长度恰好 EMBEDDING_CONTENT_LIMIT
}

#[test]
fn a_batch_never_exceeds_its_size_limit() {
    // 仓储里 40 条 pending → claim_pending_batch 被以 EMBEDDING_BATCH_SIZE 调用
}

#[test]
fn a_provider_returning_the_wrong_number_of_vectors_fails_the_batch_without_storing_anything() {
    // fake 返回 2 个向量但批里有 3 条 → 不调用 store_embedding，全部按 InvalidRequest 记失败
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::application::indexing_service`
Expected: FAIL —— `no method named process_pending_batch`。

- [ ] **Step 3: 写实现**

```rust
pub(crate) fn process_pending_batch(&self, model: &str) -> Result<BatchOutcome, RetrievalError> {
    let batch = self
        .repository
        .claim_pending_batch(SourceKind::AgentMemory, EMBEDDING_BATCH_SIZE)?;
    if batch.is_empty() {
        return Ok(BatchOutcome::default());
    }
    let inputs: Vec<String> = batch
        .iter()
        .map(|document| truncate_for_embedding(&document.content))
        .collect();

    match self.embeddings.embed(model, &inputs) {
        // 数量对不上说明 provider 的响应与请求不成对，不能靠位置把向量配给文档——
        // 错配的向量比没有向量更糟：它会安静地污染检索结果。
        Ok(vectors) if vectors.len() != batch.len() => {
            for document in &batch {
                self.repository
                    .record_failure(&document.id, FailureCategory::InvalidRequest, true)?;
            }
            Ok(BatchOutcome { succeeded: 0, failed: batch.len() })
        }
        Ok(vectors) => {
            for (document, vector) in batch.iter().zip(vectors.iter()) {
                self.repository.store_embedding(&document.id, model, vector)?;
            }
            Ok(BatchOutcome { succeeded: batch.len(), failed: 0 })
        }
        Err(failure) => {
            for document in &batch {
                let give_up = !failure.category.is_retryable()
                    || document.attempt_count + 1 >= MAX_EMBEDDING_ATTEMPTS;
                self.repository
                    .record_failure(&document.id, failure.category, give_up)?;
            }
            Ok(BatchOutcome { succeeded: 0, failed: batch.len() })
        }
    }
}

/// 按字符而非字节截断——按字节切会把多字节 UTF-8 字符劈成两半并 panic。
fn truncate_for_embedding(content: &str) -> String {
    content.chars().take(EMBEDDING_CONTENT_LIMIT).collect()
}
```

日志（设计文档 §8.2）：批次完成记 `info`，字段为批大小、耗时、成功/失败条数、模型 id；失败记 `warn`，字段为错误类别与 `attempt_count`。**不记内容、不记响应体**。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::application::indexing_service`
Expected: PASS，13 个测试（Task 7 的 5 个 + 本任务 8 个）。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/contexts/retrieval
git commit -m "feat(retrieval): batch embedding with failure-category-aware retry"
```

---

### Task 9: 检索服务与降级

**Files:**
- Create: `src-tauri/src/contexts/retrieval/application/search_service.rs`
- Modify: `src-tauri/src/contexts/retrieval/application/mod.rs`

**Interfaces:**
- Consumes: Task 3 的 `fuse_with_rrf/cosine_similarity/escape_fts_query`、Task 5 的仓储、Task 6 的配置仓储、Task 8 的 `EmbeddingPort`、Task 7 的 `IndexSourcePort`
- Produces:

```rust
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SearchOutcome {
    pub(crate) hits: Vec<ScoredHit>,
    pub(crate) degraded: Option<Degradation>,
}

impl SearchService {
    pub(crate) fn search(&self, query: &RetrievalQuery) -> Result<SearchOutcome, RetrievalError>;
}
```

`search` 只在"未配置"时返回 `Err(RetrievalError::NotConfigured)`；其余一切失败都走降级，不返回 Err。

- [ ] **Step 1: 写失败的测试**

```rust
#[test]
fn both_paths_healthy_yields_no_degradation_and_marks_overlap_as_both() { }

#[test]
fn a_hit_found_only_by_the_vector_path_is_marked_vector() { }

#[test]
fn a_hit_found_only_by_the_keyword_path_is_marked_keyword() { }

#[test]
fn query_embedding_failure_degrades_to_keyword_only_instead_of_erroring() {
    // fake embedding 返回 Err → outcome.degraded == Some(Degradation::KeywordOnly)，且 hits 非空
}

#[test]
fn keyword_path_failure_degrades_to_vector_only_instead_of_erroring() {
    // fake 仓储的 keyword_candidates 返回 Err → degraded == Some(Degradation::VectorOnly)
}

#[test]
fn both_paths_available_but_empty_is_success_not_an_error() {
    // 两路都可用、都没命中 → Ok，hits 为空，degraded 为 None
}

#[test]
fn both_paths_failing_reports_unavailable_rather_than_an_empty_result() {
    // fake embedding 返回 Err 且 fake 仓储的 keyword_candidates 也返回 Err
    // → Err(RetrievalError::Unavailable)，**不是** Ok(空列表)。
    // 这一条与上一条成对存在：区分"搜不了"和"没有"正是它们的全部意义。
    assert_eq!(service.search(&query).unwrap_err(), RetrievalError::Unavailable);
}

#[test]
fn results_are_truncated_to_the_requested_limit() { }

#[test]
fn a_hit_whose_source_row_is_gone_is_skipped_rather_than_returned_stale() {
    // 候选里有 m1、m2，但源快照只有 m1 → 结果只含 m1
}

#[test]
fn an_unconfigured_service_reports_not_configured() {
    assert_eq!(service.search(&query).unwrap_err(), RetrievalError::NotConfigured);
}

#[test]
fn the_query_is_escaped_before_reaching_fts() {
    // query 文本 `a OR b` → fake 仓储收到的 query 参数是 "\"a OR b\""
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::application::search_service`
Expected: FAIL。

- [ ] **Step 3: 写实现**

```rust
/// 双路召回 → RRF 融合 → 回查源表。
///
/// 铁律：检索失败**永不**让生成失败（设计文档 §8.1）。除"未配置"外，任何一路的失败都只是
/// 降级，因为把一个可选增强能力的故障冒泡成生成失败是不可接受的。
pub(crate) fn search(&self, query: &RetrievalQuery) -> Result<SearchOutcome, RetrievalError> {
    let configuration = self.configuration.load()?;
    let Some((_profile, model)) = configuration.resolved_model() else {
        return Err(RetrievalError::NotConfigured);
    };
    let over_fetch = query.limit.saturating_mul(4).max(query.limit);

    let vector_ranking = self.vector_ranking(query, model, over_fetch);
    let keyword_ranking = self
        .repository
        .keyword_candidates(
            &query.scope,
            SourceKind::AgentMemory,
            &escape_fts_query(&query.text),
            over_fetch,
        )
        .ok();

    // 两路都失败必须与"两路都可用但都没命中"区分开。复用已有的 Err 路径而不是新增一种
    // 降级值：Task 13 的 `execute_recall` 已有分支会把 Err 转成**成功的**工具结果
    // "检索暂时不可用"，所以 §8.1 的铁律仍然成立，且不必新增一套模型要理解的词汇。
    let degraded = match (&vector_ranking, &keyword_ranking) {
        (None, None) => return Err(RetrievalError::Unavailable),
        (None, Some(_)) => Some(Degradation::KeywordOnly),
        (Some(_), None) => Some(Degradation::VectorOnly),
        _ => None,
    };
    let vector_ids = vector_ranking.unwrap_or_default();
    let keyword_ids = keyword_ranking.unwrap_or_default();

    let fused = fuse_with_rrf(&[vector_ids.clone(), keyword_ids.clone()]);
    let in_vector: HashSet<&str> = vector_ids.iter().map(String::as_str).collect();
    let in_keyword: HashSet<&str> = keyword_ids.iter().map(String::as_str).collect();

    // 回查源表拿权威内容：索引行可能陈旧，源已删则跳过——这保证已删记忆永不外泄，
    // 也是显式撤销失败时的第一道兜底（§5.3）。
    let sources: HashMap<String, IndexSourceRecord> = self
        .source
        .snapshot()?
        .into_iter()
        .map(|record| (record.source_id.clone(), record))
        .collect();

    let hits = fused
        .into_iter()
        .filter_map(|(source_id, score)| {
            let record = sources.get(&source_id)?;
            let matched_via = match (
                in_vector.contains(source_id.as_str()),
                in_keyword.contains(source_id.as_str()),
            ) {
                (true, true) => MatchedVia::Both,
                (true, false) => MatchedVia::Vector,
                _ => MatchedVia::Keyword,
            };
            Some(ScoredHit {
                source_id,
                content: record.content.clone(),
                created_at: record.created_at.clone(),
                score,
                matched_via,
            })
        })
        .take(query.limit)
        .collect();

    Ok(SearchOutcome { hits, degraded })
}

/// `None` 表示这一路整体不可用（query embedding 失败或候选查询失败），交由调用方降级；
/// 空 `Vec` 表示这一路可用但没有命中，是正常结果。
fn vector_ranking(&self, query: &RetrievalQuery, model: &str, limit: usize) -> Option<Vec<String>> {
    let embedded = self.embeddings.embed(model, &[query.text.clone()]).ok()?;
    let query_vector = embedded.into_iter().next()?;
    let candidates = self
        .repository
        .vector_candidates(&query.scope, SourceKind::AgentMemory, model)
        .ok()?;
    let mut scored: Vec<(String, f32)> = candidates
        .into_iter()
        .filter_map(|(source_id, vector)| {
            cosine_similarity(&query_vector, &vector).map(|score| (source_id, score))
        })
        .collect();
    scored.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });
    Some(scored.into_iter().take(limit).map(|(id, _)| id).collect())
}
```

日志：检索执行记 `debug`，字段为 scope 哈希、候选集大小、两路命中数、耗时；降级记 `warn`，只记降级原因类别。**query 只记长度与哈希，绝不记原文。**

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::application::search_service`
Expected: PASS，10 个测试。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/contexts/retrieval
git commit -m "feat(retrieval): hybrid search with RRF fusion and graceful degradation"
```

---

### Task 10: openai-compatible embedding 适配器

**Files:**
- Create: `src-tauri/src/contexts/retrieval/infrastructure/openai_embedding_adapter.rs`
- Modify: `src-tauri/src/contexts/retrieval/infrastructure/mod.rs`

**Interfaces:**
- Consumes: Task 8 的 `EmbeddingPort`/`EmbeddingFailure`
- Produces: `HttpEmbeddingAdapter`，以及 `application/ports.rs` 中新增的消费侧契约 `EmbeddingEndpointPort`（设计文档 §4.3 明确它属于 `retrieval::application::ports`，**不要**放进 infrastructure；其实现由 Task 12 的 bootstrap 适配器提供）

```rust
#[derive(Debug, Clone)]
pub(crate) struct ResolvedEmbeddingEndpoint {
    pub(crate) base_url: String,
    pub(crate) credential: String,
}

pub(crate) trait EmbeddingEndpointPort: Send + Sync {
    fn resolve(&self, profile_id: &str) -> Result<ResolvedEmbeddingEndpoint, RetrievalError>;
}
```

**惯例对齐：** HTTP 走 `crate::platform::network`（`onepiece_model_discovery.rs:7-13` 已经这么用），复用代理与超时设置；`platform/network` 只保留共享的代理与凭据探测，不承载 embedding 协议细节。响应体大小上限照 `onepiece_model_discovery.rs:20` 的 `MAX_RESPONSE_BYTES`。

- [ ] **Step 1: 写失败的测试**

失败分类是纯映射，单独抽函数就能不起 HTTP 服务器地测：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_status_maps_to_its_failure_category() {
        assert_eq!(category_for_status(401), FailureCategory::Auth);
        assert_eq!(category_for_status(403), FailureCategory::Auth);
        assert_eq!(category_for_status(400), FailureCategory::InvalidRequest);
        assert_eq!(category_for_status(404), FailureCategory::InvalidRequest);
        assert_eq!(category_for_status(429), FailureCategory::RateLimit);
        assert_eq!(category_for_status(500), FailureCategory::Network);
        assert_eq!(category_for_status(503), FailureCategory::Network);
    }

    #[test]
    fn a_non_https_endpoint_is_rejected_before_any_request_is_made() {
        // 照 onepiece_model_discovery.rs:44-46 的既有约束
    }

    #[test]
    fn the_response_envelope_is_parsed_in_index_order_not_arrival_order() {
        let body = r#"{"data":[{"index":1,"embedding":[0.5]},{"index":0,"embedding":[0.25]}]}"#;
        let vectors = parse_embedding_response(body).expect("parse");
        assert_eq!(vectors, vec![vec![0.25], vec![0.5]]);
    }

    #[test]
    fn a_malformed_response_is_an_invalid_request_failure_not_a_panic() {
        assert!(parse_embedding_response("not json").is_err());
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::infrastructure::openai_embedding_adapter`
Expected: FAIL。

- [ ] **Step 3: 写实现**

请求体：`POST {base_url}/embeddings`，`{"model": model, "input": inputs}`，头部 `Authorization: Bearer <credential>`、`Accept: application/json`。

```rust
fn category_for_status(status: u16) -> FailureCategory {
    match status {
        401 | 403 => FailureCategory::Auth,
        429 => FailureCategory::RateLimit,
        400..=499 => FailureCategory::InvalidRequest,
        _ => FailureCategory::Network,
    }
}

/// provider 不保证 `data` 按请求顺序返回，必须按每项自带的 `index` 重排——
/// 否则向量会被错配到别的文档上，安静地污染检索结果。
fn parse_embedding_response(body: &str) -> Result<Vec<Vec<f32>>, EmbeddingFailure> {
    let envelope: EmbeddingEnvelope = serde_json::from_str(body).map_err(|error| EmbeddingFailure {
        category: FailureCategory::InvalidRequest,
        message: format!("malformed embedding response: {error}"),
    })?;
    let mut entries = envelope.data;
    entries.sort_by_key(|entry| entry.index);
    Ok(entries.into_iter().map(|entry| entry.embedding).collect())
}

#[derive(serde::Deserialize)]
struct EmbeddingEnvelope {
    data: Vec<EmbeddingEntry>,
}

#[derive(serde::Deserialize)]
struct EmbeddingEntry {
    #[serde(default)]
    index: usize,
    embedding: Vec<f32>,
}
```

超时、reqwest 错误 → `FailureCategory::Network`。**凭据只在进程内传给适配器，不写日志、不进错误消息。**

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::infrastructure::openai_embedding_adapter`
Expected: PASS，4 个测试。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/contexts/retrieval
git commit -m "feat(retrieval): add openai-compatible embedding adapter"
```

---

### Task 11: `agent_runtime` 侧的两条跨上下文契约

**Files:**
- Modify: `src-tauri/src/contexts/agent_runtime/application/service.rs:71-87`
- Create: `src-tauri/src/contexts/agent_runtime/application/model_category.rs`
- Modify: `src-tauri/src/contexts/agent_runtime/api.rs`
- Modify: `src-tauri/src/contexts/agent_runtime/application/mod.rs`

**Interfaces:**
- Produces（`agent_runtime::api` 新增两个方法）：

```rust
pub(crate) fn resolve_embedding_endpoint(&self, profile_id: &str)
    -> Result<EmbeddingEndpointView, AgentRuntimeApplicationError>;
pub(crate) fn list_embedding_models(&self, profile_id: &str, transient_credential: Option<&str>)
    -> Result<Vec<OnePieceProviderModelOption>, AgentRuntimeApplicationError>;

pub(crate) struct EmbeddingEndpointView {
    pub(crate) base_url: String,
    pub(crate) interface_format: String,
    pub(crate) credential: String,
}
```

**核实过的现状：** `is_chat_model`（`service.rs:71-87`）是一个排除关键词表（`embedding`/`rerank`/`whisper`/`tts`/`audio`/`image`/`moderation`/`realtime`/`sora`/`stable-diffusion`）。embedding 过滤器必须与它**由同一份判定派生**，否则两处各自维护关键词表必然漂移。

- [ ] **Step 1: 写失败的测试**

`model_category.rs` 末尾：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_and_embedding_classifications_are_mutually_exclusive() {
        for id in ["gpt-4o", "text-embedding-3-small", "bge-reranker", "whisper-1"] {
            assert!(
                !(is_chat_model(id) && is_embedding_model(id)),
                "{id} classified as both"
            );
        }
    }

    #[test]
    fn embedding_models_are_recognized_case_insensitively() {
        assert!(is_embedding_model("text-embedding-3-small"));
        assert!(is_embedding_model("TEXT-EMBEDDING-ADA-002"));
        assert!(is_embedding_model("bge-m3-embedding"));
    }

    #[test]
    fn chat_models_are_not_embedding_models() {
        for id in ["gpt-4o", "deepseek-chat", "claude-opus-4-8"] {
            assert!(!is_embedding_model(id), "{id}");
            assert!(is_chat_model(id), "{id}");
        }
    }

    #[test]
    fn non_chat_non_embedding_models_belong_to_neither() {
        for id in ["whisper-1", "dall-e-3", "bge-reranker-v2"] {
            assert!(!is_chat_model(id), "{id}");
            assert!(!is_embedding_model(id), "{id}");
        }
    }
}
```

第四个测试是关键：它锁死"embedding 过滤器不是 `is_chat_model` 的简单取反"——rerank/whisper 两者都不属于。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml agent_runtime::application::model_category`
Expected: FAIL。

- [ ] **Step 3: 写实现并把 `service.rs` 改为复用**

```rust
//! 模型类别判定的唯一真源。chat 与 embedding 两个过滤器必须从这里派生——
//! 两处各自维护关键词表，迟早会在新增模型时漂移成互相矛盾的判断。

const NON_CHAT_KEYWORDS: &[&str] = &[
    "embedding", "rerank", "whisper", "tts", "audio", "image", "moderation", "realtime", "sora",
    "stable-diffusion",
];

const EMBEDDING_KEYWORDS: &[&str] = &["embedding", "embed-"];

pub(crate) fn is_chat_model(id: &str) -> bool {
    let id = id.to_ascii_lowercase();
    !NON_CHAT_KEYWORDS.iter().any(|excluded| id.contains(excluded))
}

pub(crate) fn is_embedding_model(id: &str) -> bool {
    let id = id.to_ascii_lowercase();
    EMBEDDING_KEYWORDS.iter().any(|keyword| id.contains(keyword))
}
```

`service.rs:71-87` 的 `fn is_chat_model` **删除**，改为 `use super::model_category::is_chat_model;`。

`api.rs` 新增两个方法：`resolve_embedding_endpoint` 从已保存 Profile 解析 base_url/interface_format/凭据；`list_embedding_models` 复用现有 model discovery 的 HTTP 与凭据路径（`HttpOnePieceModelDiscoveryAdapter`），但用 `is_embedding_model` 过滤。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml agent_runtime`
Expected: PASS。既有 model discovery 测试必须全绿——`is_chat_model` 行为不能变。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/contexts/agent_runtime
git commit -m "feat(agent-runtime): derive chat and embedding filters from one model category source"
```

---

### Task 12: `retrieval::api`、bootstrap 装配与后台 worker

**Files:**
- Create: `src-tauri/src/contexts/retrieval/api.rs`
- Create: `src-tauri/src/bootstrap/retrieval.rs`
- Modify: `src-tauri/src/contexts/retrieval/mod.rs`、`src-tauri/src/bootstrap/mod.rs`

**Interfaces:**
- Produces:

```rust
impl RetrievalApi {
    pub(crate) fn search(&self, agent_id: &str, folder: Option<&str>, query: &str, limit: usize)
        -> Result<SearchOutcome, RetrievalError>;
    pub(crate) fn remove(&self, source_kind: SourceKind, source_id: &str) -> Result<(), RetrievalError>;
    pub(crate) fn is_configured(&self) -> bool;
    pub(crate) fn configuration(&self) -> Result<RetrievalConfiguration, RetrievalError>;
    pub(crate) fn save_configuration(&self, profile_id: &str, model: &str) -> Result<(), RetrievalError>;
    pub(crate) fn index_status(&self, agent_id: &str) -> Result<RetrievalIndexStatus, RetrievalError>;
    pub(crate) fn rebuild(&self, agent_id: &str) -> Result<(), RetrievalError>;
    pub(crate) fn wake_worker(&self);
}
```

`is_configured()` 必须**永不返回错误也永不阻塞**——它在每次生成的工具集解析路径上被调用（Task 13）。内部读一次配置，失败即视为未配置。

- [ ] **Step 1: 写失败的测试**

`api.rs` 的测试用 fake 仓储装配：

```rust
#[test]
fn an_unconfigured_api_reports_not_configured_without_erroring() {
    assert!(!api.is_configured());
}

#[test]
fn a_configuration_load_failure_is_treated_as_unconfigured_rather_than_propagated() {
    // fake 配置仓储返回 Err → is_configured() == false，不 panic 不 Err
}

#[test]
fn search_scopes_a_folderless_session_to_the_empty_string_sentinel() {
    // folder = None → 传给 SearchService 的 scope.folder == ""
}

#[test]
fn remove_delegates_to_the_repository_delete() { }
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval::api`
Expected: FAIL。

- [ ] **Step 3: 写实现与 bootstrap**

`bootstrap/retrieval.rs` 负责三件事：

1. 装配 `SqliteRetrievalDocumentRepository` + `SqliteRetrievalConfigurationRepository` + `HttpEmbeddingAdapter` + 记忆源适配器 + `EmbeddingEndpointPort` 适配器（后者调 `agent_runtime::api::resolve_embedding_endpoint`，**这是 `retrieval` 与 `agent_runtime` 唯一的连接点**，写在 bootstrap 里所以两个上下文互不 import 对方的 infrastructure）。
2. 注册后台 worker 线程，与既有 background jobs 同层。
3. 暴露 `RetrievalApi` 给 command 层。

worker 循环：

```rust
// 三种驱动方式：启动时跑一轮（顺带回填历史存量记忆，不需要单独的数据迁移脚本）、
// 保存记忆后的唤醒信号（不写库、不等待、失败无害）、定时兜底轮询（信号丢失时最多延迟一个周期）。
loop {
    if let Err(error) = service.reconcile() { /* warn 日志，继续 */ }
    if let Some((_, model)) = configuration.load().ok().and_then(|c| c.resolved_model().map(|(p, m)| (p.to_string(), m.to_string()))) {
        // 串行执行，不并发冲击速率限制
        while let Ok(outcome) = service.process_pending_batch(&model) {
            if outcome.succeeded == 0 && outcome.failed == 0 { break; }
        }
    }
    wait_for_signal_or_timeout(Duration::from_secs(RECONCILE_POLL_INTERVAL_SECONDS));
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml retrieval`
Expected: PASS，全部 retrieval 测试。

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 退出码 0。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/contexts/retrieval src-tauri/src/bootstrap
git commit -m "feat(retrieval): publish api facade and wire the background indexing worker"
```

---

### Task 13: `recall` 工具

**Files:**
- Modify: `src-tauri/src/contexts/agent_runtime/application/tool_catalog.rs`
- Modify: `src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs`

**Interfaces:**
- Consumes: Task 12 的 `RetrievalApi::{search, is_configured, wake_worker}`
- Produces: `RECALL_TOOL_NAME`、`recall_tool_definition()`，以及 `agent_runtime` 侧的消费契约（定义在 `agent_runtime/application/ports.rs`，实现由 Task 12 的 bootstrap 装配，从而 `agent_runtime` 不 import `retrieval` 的 infrastructure）：

```rust
pub(crate) struct AgentRetrievalHit {
    pub(crate) content: String,
    pub(crate) created_at: String,
    pub(crate) matched_via: String,
}

pub(crate) struct AgentRetrievalOutcome {
    pub(crate) hits: Vec<AgentRetrievalHit>,
    pub(crate) degraded: Option<String>,
}

pub(crate) trait AgentRetrievalPort: Send + Sync {
    /// 每次生成的工具集解析路径上都会调用，因此必须永不阻塞、永不返回错误。
    fn is_configured(&self) -> bool;
    fn search(
        &self,
        agent_id: &str,
        folder: Option<&str>,
        query: &str,
        limit: usize,
    ) -> Result<AgentRetrievalOutcome, String>;
    /// 保存记忆后的唤醒信号：不写库、不等待、失败无害。
    fn notify_source_changed(&self);
}
```

这层投影 struct 刻意不复用 `retrieval` 的 `ScoredHit`——`source_id` 与 `score` 是内部字段，不该跨上下文边界流出。

**核实过的现状（设计文档 §7.2 已据此更正）：** `resolve_tool_catalog()`（`api_process_adapter.rs:729-761`）**已经**是唯一的生产解析点，只在 `api_process_adapter.rs:475` 被调用一次。`anthropic_provider.rs:323` 与 `openai_compatible_provider.rs:309` 两处都在 `#[cfg(test)] mod tests` 内，不是生产路径——**没有"三处收口"要做**。

会被打破、必须一并更新的既有断言：`tool_catalog.rs:147`（`catalog.len() == 3`）、`tool_catalog.rs:157`（plan mode `len() == 2`）、`api_process_adapter.rs:2888`（`tools.len() == 259`）、`api_process_adapter.rs:2941`（`tools == plan_mode_tool_catalog()`）。

- [ ] **Step 1: 写失败的测试**

`tool_catalog.rs`：

```rust
#[test]
fn the_recall_tool_never_exposes_scope_to_the_model() {
    // scope 若进 schema，模型就能构造参数读别的 agent 或别的项目的记忆。这是安全边界。
    let definition = recall_tool_definition();
    let properties = definition.input_schema["properties"].as_object().expect("properties");
    assert!(properties.contains_key("query"));
    assert!(properties.contains_key("limit"));
    assert_eq!(properties.len(), 2);
    for forbidden in ["agent_id", "agentId", "folder", "scope", "project"] {
        assert!(!properties.contains_key(forbidden), "{forbidden} must not be model-supplied");
    }
    assert_eq!(definition.input_schema["required"], json!(["query"]));
}

#[test]
fn recall_auto_approves_for_the_same_reason_remember_does() {
    assert_eq!(
        risk_tier_for(RECALL_TOOL_NAME, &json!({"query": "npm"})),
        ToolRiskTier::AutoApprove
    );
}

#[test]
fn the_fixed_catalog_stays_unconditional_and_excludes_recall() {
    // tool_catalog()/plan_mode_tool_catalog() 保持纯函数、不感知配置；
    // 条件性只存在于 resolve_tool_catalog()。
    assert!(tool_catalog().iter().all(|tool| tool.name != RECALL_TOOL_NAME));
    assert!(plan_mode_tool_catalog().iter().all(|tool| tool.name != RECALL_TOOL_NAME));
}
```

`api_process_adapter.rs`：

```rust
#[test]
fn resolve_tool_catalog_omits_recall_when_retrieval_is_not_configured() { }

#[test]
fn resolve_tool_catalog_offers_recall_when_retrieval_is_configured() { }

#[test]
fn plan_mode_offers_recall_when_configured_because_planning_needs_history_most() { }

#[test]
fn recall_returns_a_successful_result_when_retrieval_fails_so_generation_continues() {
    // fake RetrievalApi 返回 Err → outcome.is_error == false，output 告知模型检索暂时不可用
}

#[test]
fn recall_scope_comes_from_the_session_not_from_model_input() {
    // 模型传 {"query":"x","agent_id":"other"} → fake 收到的 agent_id 仍是会话自身的
}

#[test]
fn recall_clamps_its_limit_to_the_documented_bounds() {
    // limit 缺省 → 5；limit = 0 → 1；limit = 999 → 20
}

#[test]
fn recall_projects_away_internal_fields() {
    // 返回体只含 content / created_at / matched_via，不含 source_id 与 score
}

#[test]
fn recall_surfaces_degradation_only_when_degraded() {
    // 正常 → 无 degraded 键；降级 → degraded == "keyword_only"
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml agent_runtime`
Expected: FAIL。

- [ ] **Step 3: 写实现**

`tool_catalog.rs`：

```rust
pub(crate) const RECALL_TOOL_NAME: &str = "recall";

/// scope（agent id 与工作区文件夹）**刻意不进 schema**：它由运行时从会话上下文注入，模型无法
/// 指定——否则模型可构造参数读取其他 agent 或其他项目的记忆。这是安全边界，不是省事。
pub(crate) fn recall_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: RECALL_TOOL_NAME.to_string(),
        description: "Search your saved memories for this project by meaning, not just keywords. Use when the user refers to something from an earlier session, or when you need context that isn't in the current conversation.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "What to look for, in natural language."
                },
                "limit": {
                    "type": "integer",
                    "description": "How many memories to return. Defaults to 5, capped at 20."
                }
            },
            "required": ["query"]
        }),
    }
}
```

`risk_tier_for` 增加一条 match 臂，理由与 `remember` 同源（`tool_catalog.rs:120-123`）：

```rust
        // 只读本应用自身存储，不触碰用户文件系统，不执行任何外部动作。唯一新增出网面是 query
        // 文本发往 embedding provider，而索引时记忆内容本就已经发出过，不构成新增暴露面。
        RECALL_TOOL_NAME => ToolRiskTier::AutoApprove,
```

`resolve_tool_catalog` 增加 `retrieval_available: bool` 形参，两个分支都注入：

```rust
    if plan_mode {
        let mut tools = plan_mode_tool_catalog();
        // plan mode 同样提供 recall：只读，且规划阶段最需要历史上下文。
        if retrieval_available {
            tools.push(recall_tool_definition());
        }
        return tools;
    }
    let mut tools = tool_catalog();
    if retrieval_available {
        tools.push(recall_tool_definition());
    }
```

`execute_tool_call` 在 `REMEMBER_TOOL_NAME` 分支旁增加 `recall`（同样在工作区文件夹门禁**之前**，理由相同：只碰本应用存储）：

```rust
    if name == RECALL_TOOL_NAME {
        return execute_recall(input, agent_id, workspace_folder, retrieval);
    }
```

```rust
/// 检索失败时**不返回 Err**，而是返回正常的工具结果告知模型"检索暂时不可用"，让生成继续。
/// 把可选增强能力的故障冒泡成生成失败是不可接受的（设计文档 §8.1）。
fn execute_recall(
    input: &Value,
    agent_id: &str,
    workspace_folder: Option<&str>,
    retrieval: &dyn AgentRetrievalPort,
) -> ToolExecutionOutcome {
    let query = input.get("query").and_then(Value::as_str).unwrap_or_default().trim();
    if query.is_empty() {
        return ToolExecutionOutcome {
            output: "No query was provided to recall.".to_string(),
            is_error: true,
        };
    }
    let limit = input
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .clamp(1, 20) as usize;
    match retrieval.search(agent_id, workspace_folder, query, limit) {
        Ok(outcome) => ToolExecutionOutcome {
            output: serde_json::to_string(&recall_payload(&outcome)).unwrap_or_else(|_| "{\"results\":[]}".to_string()),
            is_error: false,
        },
        Err(_) => ToolExecutionOutcome {
            output: "Memory search is temporarily unavailable. Continue without it.".to_string(),
            is_error: false,
        },
    }
}
```

`recall_payload` 只投影 `content` / `created_at` / `matched_via`——`source_id` 与 `score` 是内部字段，对模型没有决策价值，只增加 token 消耗并为幻觉提供素材。`degraded` 仅在降级时出现。

`agent_runtime` 通过一个 `AgentRetrievalPort` trait 消费 `retrieval::api`，实现在 bootstrap 装配（保持 `agent_runtime` 不 import `retrieval` 的 infrastructure）。

- [ ] **Step 4: 更新被打破的既有断言**

- `tool_catalog.rs:147` / `:157`：`tool_catalog()` 与 `plan_mode_tool_catalog()` 本身不变，长度断言保持 3 / 2，另加 Step 1 的第三个新测试。
- `api_process_adapter.rs:2888`：该用例用 `retrieval_available = false` 调用，`259` 不变；另加一个 `true` 的用例断言 `260` 且末位是 `recall`。
- `api_process_adapter.rs:2941`：改为 `assert_eq!(tools, plan_mode_tool_catalog())` 仅在 `retrieval_available = false` 时成立，配置时另断言多一个 `recall`。

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS，全绿。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/contexts/agent_runtime
git commit -m "feat(agent-runtime): add the conditional recall tool with runtime-injected scope"
```

---

### Task 14: 记忆变更时的索引挂钩（删除撤销 + 保存唤醒）

设计文档 §5.1 的三种 worker 驱动方式中，"启动时跑一轮"与"定时兜底轮询"在 Task 12 已实现，第三种"保存记忆后发唤醒信号"在这里补齐；§5.3 的删除撤销也在这里。两者都是"记忆表变更 → 通知索引"的同一类挂钩，同一个 reviewer 一次看完。

**Files:**
- Modify: `src-tauri/src/commands/agent_runtime/delete_agent_memory.rs`
- Modify: `src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs`（`execute_remember` 保存成功后发信号）
- Modify: `src-tauri/src/contexts/agent_runtime/api.rs`

**Interfaces:**
- Consumes: Task 12 的 `RetrievalApi::{remove, wake_worker}`；Task 13 的 `AgentRetrievalPort::notify_source_changed`

- [ ] **Step 1: 写失败的测试**

```rust
#[test]
fn deleting_a_memory_revokes_its_retrieval_index_entry() {
    let retrieval = FakeRetrieval::default();

    delete_agent_memory_with(&memories, &retrieval, "m1").expect("delete");

    assert_eq!(*retrieval.removed.lock().expect("lock"), vec!["m1".to_string()]);
}

#[test]
fn a_failed_revocation_does_not_fail_the_delete() {
    // 两道兜底让这里可以安全地吞掉错误：检索侧回查源表保证已删记忆不会被返回，
    // reconcile 的孤儿清理最终删掉残留行。
    let retrieval = FakeRetrieval { remove_fails: true, ..FakeRetrieval::default() };

    let result = delete_agent_memory_with(&memories, &retrieval, "m1");

    assert!(result.is_ok());
}

#[test]
fn saving_a_memory_wakes_the_indexing_worker() {
    let retrieval = FakeRetrieval::default();

    let outcome = execute_remember(
        &json!({"content": "Uses npm."}),
        "test-agent",
        None,
        &memories,
        &retrieval,
    );

    assert!(!outcome.is_error);
    assert_eq!(retrieval.wake_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn a_rejected_memory_does_not_wake_the_worker() {
    // 空内容被拒 → 没有新记忆要索引，不该白唤醒一轮
    let retrieval = FakeRetrieval::default();

    let outcome = execute_remember(&json!({"content": "   "}), "test-agent", None, &memories, &retrieval);

    assert!(outcome.is_error);
    assert_eq!(retrieval.wake_calls.load(Ordering::SeqCst), 0);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml agent_runtime`
Expected: FAIL。

- [ ] **Step 3: 写实现**

删除侧：删除成功后调用 `retrieval.remove(SourceKind::AgentMemory, memory_id)`，失败只记 `warn` 日志，不改变命令返回值。

保存侧：`execute_remember`（`api_process_adapter.rs:1288-1315`）在 `memories.save(...)` 返回 `Ok` 之后调 `retrieval.notify_source_changed()`。该调用**不写库、不等待、失败无害**——它只是把兜底轮询的最长延迟从一个周期缩短到近实时；信号丢了也只是慢一点，绝不能反过来让保存失败。因此签名返回 `()`，不返回 `Result`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml agent_runtime`
Expected: PASS，4 个新测试全绿，既有 `execute_tool_call_routes_remember_*` 测试（`api_process_adapter.rs:2623`、`:2647`）也必须仍然通过。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/commands src-tauri/src/contexts/agent_runtime
git commit -m "feat(agent-runtime): hook memory saves and deletes into retrieval indexing"
```

---

### Task 15: Tauri commands 与前端服务边界

**Files:**
- Create: `src-tauri/src/commands/retrieval/mod.rs` 及 5 个命令文件（每命令一文件，按功能域分组）
- Modify: `src-tauri/src/lib.rs`（注册命令）
- Modify: `src/types/agent.ts`
- Modify: `src/services/agent-service.ts`
- Modify: `src/services/tauri-agent-client.ts`
- Modify: `src/services/web-agent-client.ts`
- Test: `src/services/web-agent-client.test.ts`

**Interfaces:**
- Produces（设计文档 §7.4，两个 client 必须同时实现）：

```ts
getRetrievalConfiguration(): Promise<RetrievalConfiguration>;
saveRetrievalConfiguration(profileId: string, modelId: string): Promise<void>;
listEmbeddingModels(profileId: string, transientCredential?: string): Promise<EmbeddingModelOption[]>;
getRetrievalIndexStatus(agentId: string): Promise<RetrievalIndexStatus>;
rebuildRetrievalIndex(agentId: string): Promise<void>;
```

配置是全局单例；状态与重建按 agent 聚合该 agent 名下**所有** `scope_folder` 的行。

```ts
export interface RetrievalConfiguration {
  sourceProfileId: string | null;
  embeddingModel: string | null;
}

export interface RetrievalIndexStatus {
  indexed: number;
  pending: number;
  failed: number;
  lastFailureCategory: string | null;
}

export interface EmbeddingModelOption {
  id: string;
  displayName: string;
}
```

- [ ] **Step 1: 写失败的测试**

`src/services/web-agent-client.test.ts` 追加：

```ts
it('returns an unconfigured retrieval configuration by default', async () => {
  const client = createWebAgentClient();
  await expect(client.getRetrievalConfiguration()).resolves.toEqual({
    sourceProfileId: null,
    embeddingModel: null,
  });
});

it('round-trips a saved retrieval configuration', async () => {
  const client = createWebAgentClient();
  await client.saveRetrievalConfiguration('profile-a', 'text-embedding-3-small');
  await expect(client.getRetrievalConfiguration()).resolves.toEqual({
    sourceProfileId: 'profile-a',
    embeddingModel: 'text-embedding-3-small',
  });
});

it('reports index status without issuing any network request', async () => {
  const fetchSpy = vi.spyOn(globalThis, 'fetch');
  const client = createWebAgentClient();
  await expect(client.getRetrievalIndexStatus('agent-1')).resolves.toEqual({
    indexed: expect.any(Number),
    pending: expect.any(Number),
    failed: expect.any(Number),
    lastFailureCategory: null,
  });
  expect(fetchSpy).not.toHaveBeenCalled();
});

it('rebuilding the index resets failures in the mock runtime', async () => {
  const client = createWebAgentClient();
  await client.rebuildRetrievalIndex('agent-1');
  const status = await client.getRetrievalIndexStatus('agent-1');
  expect(status.failed).toBe(0);
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `env -u all_proxy -u ALL_PROXY npm run test -- web-agent-client`
Expected: FAIL —— `client.getRetrievalConfiguration is not a function`。

- [ ] **Step 3: 写实现**

Rust 侧 5 个命令，签名一律 `Result<T, String>`：`get_retrieval_configuration`、`save_retrieval_configuration`、`list_embedding_models`、`get_retrieval_index_status`、`rebuild_retrieval_index`。在 `lib.rs` 的 `invoke_handler` 中注册。

`agent-service.ts` 接口追加 5 个方法；`tauri-agent-client.ts` 用 `invoke()` 实现；`web-agent-client.ts` 用内存 mock 实现，**不发任何网络请求**。

- [ ] **Step 4: 运行测试确认通过**

Run: `env -u all_proxy -u ALL_PROXY npm run test -- web-agent-client`
Expected: PASS。

Run: `npm run lint`
Expected: 无 error。

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/commands src-tauri/src/lib.rs src/types src/services
git commit -m "feat(retrieval): expose retrieval configuration across the service boundary"
```

---

### Task 16: 配置区块 UI

**Files:**
- Create: `src/settings/pages/agents/onepiece-retrieval-section.tsx`
- Create: `src/settings/pages/agents/onepiece-retrieval-section.test.tsx`
- Modify: `src/settings/pages/agents/onepiece-configuration-panel.tsx`

新建独立文件而非塞进 `onepiece-configuration-panel.tsx`（现 133 行）：区块含 Profile 选择、模型发现、状态展示、重建动作四块状态，并进去会逼近 300 行上限。

**Interfaces:**
- Consumes: Task 15 的 5 个 service 方法

- [ ] **Step 1: 写失败的测试**

```tsx
it('lists only openai-compatible profiles as embedding sources', () => {
  // Anthropic 没有 embeddings API——列出 anthropic profile 只会让用户配出一个必然失败的组合
});

it('stays visible but not configurable when no openai-compatible profile exists', () => {
  // 与 onepiece-native-agent spec "未配置时仍可见且给出可操作状态"的既有做法一致，不隐藏
  expect(screen.getByText(/需要一个 openai-compatible/)).toBeInTheDocument();
});

it('renders indexed, pending, and failed counts', () => { });

it('shows only the failure category, never raw error text', () => {
  // 原始错误可能含凭据或 provider 响应内容
});

it('requeues everything when rebuild is confirmed', async () => {
  // 断言 rebuildRetrievalIndex 被调用，且随后重新拉取状态
});

it('loads embedding models through the service boundary, never invoke()', async () => {
  // 组件禁止直接调 Tauri invoke()
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `env -u all_proxy -u ALL_PROXY npm run test -- onepiece-retrieval-section`
Expected: FAIL —— 模块不存在。

- [ ] **Step 3: 写实现**

函数组件 + Hooks，Tailwind 类名，无内联 style，文件保持 300 行以内。四块内容：来源 Profile 选择器（只列 `interfaceFormat === 'openai-compatible'`）、embedding 模型选择器（调 `listEmbeddingModels`）、索引状态（已索引/待索引/失败 + 最近失败类别）、重建索引按钮。

在 `onepiece-configuration-panel.tsx` 中挂载该区块。

- [ ] **Step 4: 运行测试确认通过**

Run: `env -u all_proxy -u ALL_PROXY npm run test -- onepiece-retrieval-section`
Expected: PASS，6 个测试。

Run: `npm run lint`
Expected: 无 error。

- [ ] **Step 5: 提交**

```bash
git add src/settings/pages/agents
git commit -m "feat(settings): add the OnePiece retrieval configuration section"
```

---

### Task 17: E2E 与全量验收

**Files:**
- Create: `tests/e2e/onepiece-retrieval.spec.ts`

**注意（本机环境）：** Playwright CLI 在本机需要去掉 SOCKS5 代理变量才能跑，所有 `npx playwright` 命令必须加 `env -u all_proxy -u ALL_PROXY` 前缀。

- [ ] **Step 1: 写 E2E**

一条即可（设计文档 §9.6），走 mock adapter，**不打真实 API**：

```ts
test('configuring an embedding source moves the index from pending to indexed', async ({ page }) => {
  // 1. 打开 OnePiece 配置页的"检索"区块
  // 2. 选择一个 openai-compatible profile 与一个 embedding 模型并保存
  // 3. 触发索引
  // 4. 断言状态由 pending 变为 indexed
});
```

- [ ] **Step 2: 运行 E2E**

Run: `env -u all_proxy -u ALL_PROXY npx playwright test tests/e2e/onepiece-retrieval.spec.ts`
Expected: PASS，1 passed。

- [ ] **Step 3: 全量验收（AGENTS.md 规定，必须全部跑通）**

```bash
npm run lint
npm run test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
openspec validate --specs --strict
```

每条都必须看到实际输出确认通过——**不要凭"应该没问题"下结论**。任何一条失败就停下来修，不要继续。

- [ ] **Step 4: 勾掉 OpenSpec tasks 并记录实现验证结果**

更新 `openspec/changes/add-retrieval-vector-search/tasks.md` 的全部勾选框，并在其中记录 Step 3 六条命令的实际结果。

- [ ] **Step 5: 提交**

```bash
git add tests/e2e openspec/changes/add-retrieval-vector-search
git commit -m "test(retrieval): add end-to-end coverage and record verification results"
```

---

## 附：本计划相对设计文档的两处更正

实现时以本计划为准，两处已同步回设计文档：

1. **§7.2 "工具集解析收口"的前提是误读。** `resolve_tool_catalog()`（`api_process_adapter.rs:729-761`）已经是唯一的生产解析点。设计文档原先列举的另两处 `anthropic_provider.rs:323`、`openai_compatible_provider.rs:309` 都在 `#[cfg(test)] mod tests` 内。没有"三处收口"的既有隐患要修，只需在既有单一函数里注入条件性 `recall`（Task 13）。

2. **§6 的 FTS 转义没有现成 helper 可复用。** 仓库里唯一的 FTS 消费方 `contexts/workspaces/infrastructure/output_search.rs:36-47` 把原始串直接塞进 `MATCH ?1`，只挡空串与超长。`recall` 的 query 由模型自由生成，必须自己实现转义（Task 3）。

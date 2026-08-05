# OnePiece 向量检索 · 第 1 期设计

日期：2026-08-05
分支：`worktree-onepiece-vector-search`
状态：设计已确认，待转实现计划

## 1. 背景

OnePiece 的跨会话记忆（`agent-cross-session-memory`）目前把记忆按时间倒序、在 4000 字符预算内注入 system prompt。`src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs:909-914` 的注释写明这是"real retrieval 的有界替代品"，并记录 design.md 刻意推迟了 vector search/embeddings。本设计接替这个被推迟的决定。

仓库现状中三条对设计有决定性影响的事实：

- FTS5 已在用：`session_message_fts`（`migrations.rs:278`）与 `terminal_output_fts`（`remote_terminal_schema.rs:118`），都采用 external-content 表 + trigger + `tokenize='trigram'` 的写法。关键词检索不必从零建。
- Provider 目录 25 家、32 个 `openai-compatible` 端点与 16 个 `anthropic` 端点（`src/config/onepiece-provider-catalog.json`）。**Anthropic 没有 embeddings API**。
- 现有模型发现主动过滤掉 embedding 类模型（`service.rs:71-87` 的 `is_chat_model`），不能直接复用。

## 2. 范围

整体能力拆成四期，本设计只覆盖第 1 期。

| 期 | 内容 |
| --- | --- |
| **1（本期）** | 检索内核 + 跨会话记忆索引 + 模型主动调用的 `recall` 工具 |
| 2 | 会话历史（`messages`）索引 |
| 3 | 本地文档/代码知识库索引 |
| 4 | 自动注入替换现有 recency 注入 + 用户搜索 UI |

第 2/3 期只是往同一个检索内核里接新的数据源，第 4 期只是加新的消费口——内核只实现一次。

### Non-goals

明确不做，防止实现期范围蔓延：

- 不做分块（chunking）
- 不做 rerank 模型
- 不做 ANN 索引（HNSW 等）
- 不做 embedding 模型自动选择
- **不改现有 recency 注入链路**（第 4 期）
- 不索引 `messages` 与本地文件（第 2/3 期）

## 3. 已确认的关键决策

| 决策点 | 选择 | 理由 |
| --- | --- | --- |
| embedding 来源 | `EmbeddingPort` 抽象 + 远程 openai-compatible 实现 | 复用现有 provider catalog 与凭据体系，无新增重量级依赖；本地模型日后可插 |
| 向量存储 | SQLite BLOB + Rust 暴力余弦 | 单 agent 百~千条量级，扫描耗时远低于一次 embedding 网络往返；零跨平台构建风险 |
| 索引时机 | 后台异步补齐 | 保存路径不得因网络失败而失败 |
| 排序 | 向量 + FTS5 双路召回，RRF 融合 | 专有名词/路径/错误码这类字面 query 召回明显更好；embedding 不可用时天然降级 |
| 配置入口 | OnePiece 配置页新增"检索"区块，复用已保存 Profile | 不必新建一套完整的 Profile CRUD 与凭据管理 |
| 上下文归属 | 新建 `retrieval` peer context | 第 2/3 期要索引 `sessions` 与 `workspaces` 拥有的数据，内核留在 `agent_runtime` 会形成反向依赖 |

新增 peer context 需要一次架构决策并更新 `openspec/project.md:48-58` 的上下文表。

## 4. 架构

### 4.1 模块布局

```
src-tauri/src/contexts/retrieval/
├─ domain/
│  ├─ document.rs      # RetrievalDocument / SourceKind / IndexState / ContentHash
│  ├─ query.rs         # RetrievalQuery / RetrievalScope / ScoredHit
│  ├─ fusion.rs        # RRF 融合，纯函数不碰 I/O
│  └─ error.rs         # RetrievalError
├─ application/
│  ├─ ports.rs         # EmbeddingPort（算向量）/ EmbeddingEndpointPort（要端点与凭据）
│  │                   # RetrievalDocumentRepository / RetrievalConfigurationRepository
│  ├─ indexing_service.rs
│  └─ search_service.rs
├─ infrastructure/
│  ├─ schema.rs
│  ├─ sqlite_repository.rs
│  └─ openai_embedding_adapter.rs
└─ api.rs              # 跨上下文契约：index / remove / search
```

`agent_runtime` 只通过 `retrieval::api` 交互，不 import 其 repository 或 infrastructure（`project.md:63`）。

embedding 的 HTTP 适配器放在 `retrieval/infrastructure/`，与现有 `onepiece_model_discovery.rs` 同一惯例，复用 `platform/network/proxy.rs`；`platform/network` 只保留共享的代理与凭据探测，不承载 embedding 协议细节。

后台 worker 注册在 `bootstrap/`，与现有 background jobs 同层（`project.md:71`）。

### 4.2 数据模型（迁移 42 `retrieval-vector-index`）

```sql
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

-- 单例配置行。retrieval 拥有自己的配置表，而不是借用 desktop 上下文的 settings KV 表，
-- 避免为读一条自有配置去依赖另一个上下文的 api。
CREATE TABLE IF NOT EXISTS retrieval_configuration (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    source_profile_id TEXT,
    embedding_model TEXT,
    updated_at TEXT NOT NULL
);
```

配套 insert / delete / update 三个 trigger 与一次 `rebuild`，写法照搬 `migrations.rs:285-301` 的 `messages_fts_*`。

`index_state` 取值：`pending` | `indexed` | `failed`。
`source_kind` 第 1 期只有 `agent_memory`；第 2/3 期扩展为 `session_message`、`workspace_file`。
`id` 取 `format!("{source_kind}:{source_id}")`——与 `UNIQUE (source_kind, source_id)` 同源的确定性主键，reconcile 因此可以直接 upsert，不必先查后插。
`content_hash` 为 `sha2::Sha256`（`Cargo.toml:44` 已有该依赖）对 `content` UTF-8 字节的十六进制摘要，小写无分隔符。
`embedding` 为 f32 little-endian 字节序列。
`scope_folder` 沿用空串哨兵表示"无工作区文件夹"，与 `agent_memories` 一致（`memory_schema.rs:4-6`）。

四个关键取舍：

1. **scope 冗余进本表**。检索先按 `scope_agent_id + scope_folder` 过滤再暴力扫描，只反序列化候选集——这是暴力余弦成立的前提。
2. **FTS 建在 `retrieval_documents` 而非 `agent_memories`**。第 2/3 期源表不同，统一在本表做 FTS，混合检索只实现一次。
3. **模型与维度存在行上**。换 embedding 模型时不清库：检索**只用 `embedding_model` 等于当前配置模型的行做向量召回**，不匹配的行降级走 FTS 并被后台重新入队逐步收敛。既避免换模型瞬间打爆 API 配额，也杜绝不同维度的向量进入同一次余弦比较。
4. **不建到 `agent_memories` 的外键**。跨期源表不同，靠 `source_kind + source_id` 逻辑关联。检索只返回引用，由消费方回查源表——源已删则跳过，陈旧索引不会泄露内容。

### 4.3 跨上下文依赖方向

Profile、凭据、provider 目录全部由 `agent_runtime` 拥有。`retrieval` 需要它们才能调 embedding API，但 `project.md:63` 禁止跨上下文 import 对方的 infrastructure。依赖方向如下：

```
retrieval::application::ports::EmbeddingEndpointPort   （消费侧契约，retrieval 定义）
        ↑ 实现
bootstrap 里的适配器 ──→ agent_runtime::api
                          ├─ resolve_embedding_endpoint(profile_id)
                          │    → { base_url, interface_format, credential }
                          └─ list_embedding_models(profile_id, transient_credential?)
```

两条 `agent_runtime::api` 是本期需要新增的跨上下文契约：

- `resolve_embedding_endpoint` 返回调用 `/embeddings` 所需的端点与凭据。凭据只在进程内传递给适配器，不经过前端，也不写日志。
- `list_embedding_models` 复用现有 model discovery 的 HTTP 与凭据路径，但过滤条件与 `is_chat_model`（`service.rs:71-87`）相反：只保留 embedding 类模型。两个过滤器应当由同一份模型类别判定派生，避免两处各自维护关键词表而漂移。

`retrieval` 因此不知道 Profile、凭据存储、provider 目录的存在，只知道"给我一个可用的 embedding 端点"。

## 5. 索引流水线

### 5.1 以 reconcile 为真源，不做双写

`SqliteAgentMemoryRepository::save` 不做任何修改。不在保存路径里插索引行——那会引入"入队写失败 → 该记忆永远搜不到"的静默漏洞。

真源是一次差集协调，包含三类待办：

1. **新增**：`agent_memories` 中存在、`retrieval_documents` 中缺失 → 建行并置 `pending`
2. **失效**：`content_hash` 与源不匹配 → 重置为 `pending`
3. **孤儿**：`retrieval_documents` 中存在、源已不存在 → 删行

第 3 类是 §5.3 显式撤销失败时的兜底。少了它，撤销调用一旦失败，索引行会永久残留。

worker 由三种方式驱动：

- **启动时跑一轮**：顺带完成历史存量记忆的回填，不需要单独的数据迁移脚本
- **保存记忆后发唤醒信号**：不写库、不等待、失败无害
- **定时兜底轮询**：默认 5 分钟，信号丢失时最多延迟一个周期

### 5.2 worker 批处理规则

- 每批最多 32 条，使用 `/embeddings` 的批量 input
- 失败分类决定重试策略：
  - `auth` / `invalid_request` → 直接标 `failed`，不重试（无效凭据上重试只会烧配额）
  - `network` / `rate_limit` → 退避重试，间隔 1s / 4s / 15s / 60s / 300s，第 5 次仍失败则标 `failed`
- 串行执行，不并发冲击速率限制
- 单条内容超过 8000 字符时，embedding 前截断到 8000；FTS 仍索引全文

以上四个数值（批大小 32、轮询 5 分钟、重试 5 次、截断 8000）是可调常量，集中定义在一处，不散落在调用点。

### 5.3 删除

`agent_runtime` 删除记忆时调用 `retrieval::api::remove(source_kind, source_id)` 撤销索引。该调用失败有两道兜底：检索侧的回查机制保证已删记忆不会被返回，reconcile 的孤儿清理保证残留行最终被删除。

## 6. 检索流程

```
输入：query 文本、scope(agent_id, folder)、limit

├─ 向量路：embedding(query)
│          SELECT id, source_id, embedding FROM retrieval_documents
│          WHERE source_kind='agent_memory' AND scope 匹配
│            AND index_state='indexed' AND embedding_model = 当前配置模型
│          → 余弦 → top 4×limit
└─ 关键词路：FTS5 MATCH（query 需做 trigram-safe 转义）→ bm25 → top 4×limit
      ↓
RRF 融合：score = Σ 1/(60 + rank_i)
      ↓
截断到 limit → 回查 agent_memories 取权威 content（源不存在则跳过）
      ↓
ScoredHit { source_id, content, created_at, score, matched_via: vector|keyword|both }
```

向量路的 SQL 刻意不取 `content`，只拉 `embedding` BLOB，内容统一由回查提供。千条 × 1536 维 f32 约 6MB，反序列化加点积为毫秒级，相对一次 embedding 网络往返可忽略。

关键词路的 query 必须**整体转义成一个 FTS5 字符串字面量**：内部的 `"` 双写，再用 `"` 把整串包起来。仓库里唯一的 FTS 消费方 `contexts/workspaces/infrastructure/output_search.rs:36-47` 是把原始串直接塞进 `MATCH ?1` 的，只挡了空串与超长，没有可复用的转义函数。这里不能照抄：`recall` 的 query 由模型自由生成，含 `"` `*` `:` `-` `OR` `NEAR` 时 FTS5 会按查询语法解析，轻则语义跑偏，重则整条语句报错。转义后整串按短语匹配，在 trigram tokenizer 下正是想要的行为。转义函数放在 `domain/query.rs`，纯函数单测覆盖上述特殊字符。

### 6.1 降级矩阵

| 情况 | 行为 |
| --- | --- |
| 未配置 embedding | `recall` 工具不注册；现有 recency 注入照常工作 |
| 已配置但 query embedding 失败 | 只走关键词路，结果标 `degraded: keyword_only`，不报错 |
| 关键词路自身失败（FTS 语法或 IO 错误） | 只走向量路，结果标 `degraded: vector_only`，不报错 |
| 大量 pending 未索引完 | 关键词路照常——FTS 由 trigger 实时维护，不依赖 worker |
| 换了 embedding 模型 | 向量路只覆盖已收敛的行，其余走关键词，后台逐步补齐 |
| 两路都可用但都没命中 | 返回空列表，不是错误 |
| 两路都失败 | 报告"检索不可用"而非空结果——把"搜不了"说成"没有"，会让模型据此断定用户从没提过某事。复用已有错误路径：`search` 返回 `Err`，`recall` 的既有分支把它转成成功的工具结果"检索暂时不可用" |

FTS 独立于 embedding 可用，因此"未配置 embedding 就不注册工具"表面上浪费了关键词能力。这是刻意的：注册一个语义召回能力为零的 `recall` 会误导模型。**未配置 = 不注册；已配置但临时故障 = 降级关键词**。

## 7. 对外契约

### 7.1 模型工具 `recall`

```
name: recall
description: Search your saved memories for this project by meaning, not just keywords.
             Use when the user refers to something from an earlier session, or when you
             need context that isn't in the current conversation.
input_schema:
  query  (string, required)
  limit  (integer, optional) — default 5, max 20
```

**scope 不进 schema**。`agent_id` 与 `folder` 由运行时从会话上下文注入，模型无法指定——否则模型可构造参数读取其他 agent 或其他项目的记忆。这是安全边界，必须作为 spec requirement 固化。

返回体：

```json
{
  "results": [{ "content": "...", "created_at": "...", "matched_via": "both" }],
  "degraded": "keyword_only"
}
```

`degraded` 仅在降级时出现，取值 `keyword_only` | `vector_only`（见 §6.1）。工具返回体是 §6 中 `ScoredHit` 的投影：`source_id` 与 `score` 是内部字段，不进入工具结果——它们对模型没有决策价值，只增加 token 消耗并为幻觉提供素材。

风险层级 `ToolRiskTier::AutoApprove`，理由与 `remember` 同源（`tool_catalog.rs:120-123`）：只读本应用自身存储，不触碰用户文件系统，不执行任何外部动作。唯一新增出网面是 query 文本发往 embedding provider，而这在"索引时记忆内容已发出"的前提下不构成新增暴露面。

plan mode 同样提供 `recall`：只读，且规划阶段最需要历史上下文。

### 7.2 工具集解析注入点

工具集解析**已经是收口的**：`resolve_tool_catalog()`（`api_process_adapter.rs:729-761`）是唯一的生产解析点，只在 `api_process_adapter.rs:475` 被调用一次，plan mode 与 MCP 扩展都在它内部分支。`anthropic_provider.rs:323` 与 `openai_compatible_provider.rs:309` 虽然直接调 `tool_catalog()`，但两处都在 `#[cfg(test)] mod tests` 内，只是为断言请求体形状造一份样例工具列表，不是生产路径。**因此本期没有"三处收口"的既有隐患要修**（早期设计稿的这一判断是误读，已更正）。

条件性 `recall` 的注入点因此唯一：`resolve_tool_catalog()`。它的两个分支都要注入——plan mode 分支（§7.1 要求 plan mode 同样提供 `recall`）与常规分支。`tool_catalog()` 与 `plan_mode_tool_catalog()` 保持纯函数、保持无条件，不感知检索配置；可用性判断只存在于 `resolve_tool_catalog()`。

引入条件性工具会打破几处断言"目录固定长度"的既有测试，必须一并更新：`tool_catalog.rs:147`（`catalog.len() == 3`）、`tool_catalog.rs:157`（`plan_mode` 目录 `len() == 2`）、`api_process_adapter.rs:2888`（`tools.len() == 259`）、`api_process_adapter.rs:2941`（`tools == plan_mode_tool_catalog()`）。前两处因 `tool_catalog()` 本身不变而只需补"未配置时不含 recall"的新用例；后两处需按是否配置检索分别断言。

### 7.3 配置区块

OnePiece 配置页新增"检索"区块：

- **来源 Profile 选择器**：只列 `interfaceFormat = openai-compatible` 的已保存 Profile
- **embedding 模型选择器**：走 §4.3 的 `agent_runtime::api::list_embedding_models`，即复用现有 model discovery 的 HTTP 与凭据路径但反转过滤条件
- **索引状态**：已索引 / 待索引 / 失败条数 + 最近失败类别（只给类别，不展示原始错误文本）
- **重建索引**动作：清空 `failed` 与 `attempt_count`，全部重新入队

无任何 openai-compatible Profile 时（Anthropic-only 用户），区块显示为不可配置并说明原因，而非隐藏——与 `onepiece-native-agent` spec"未配置时仍可见且给出可操作状态"的既有做法一致。

### 7.4 前端服务边界

`src/services/agent-service.ts` 新增，`tauri-agent-client.ts` 与 `web-agent-client.ts` 必须同时实现：

```ts
getRetrievalConfiguration(): Promise<RetrievalConfiguration>
saveRetrievalConfiguration(profileId: string, modelId: string): Promise<void>
listEmbeddingModels(profileId: string, transientCredential?: string): Promise<ModelOption[]>
getRetrievalIndexStatus(agentId: string): Promise<RetrievalIndexStatus>
rebuildRetrievalIndex(agentId: string): Promise<void>
```

配置是全局单例（§4.2 的 `retrieval_configuration`），而状态与重建按 agent 划分：后两个方法聚合该 `agentId` 名下**所有 `scope_folder`** 的行，不再按工作区文件夹切一层。

### 7.5 Web/mock 对等边界

`agent-cross-session-memory` spec 第 72-78 行要求 Web runtime 对等模拟。RRF 融合与余弦都在 Rust 侧，Web 无法真正等价。因此 spec 必须写明：

- Web 侧**保证契约形状与可观察行为对等**：相同的结果结构、相同的 `degraded` 语义、相同的"空结果不是错误"、同样不发任何网络请求
- Web 侧**不保证排序算法等价**：mock 使用简单的词重叠打分

含糊带过这一条，实现时必然有人试图在 TypeScript 里复刻余弦。

## 8. 错误处理与日志

### 8.1 铁律：检索失败永不让生成失败

`retrieval` 内部使用 `RetrievalError` enum，在 `api.rs` 边界转换；跨 Tauri command 边界按 AGENTS.md 转为 `Result<T, String>`。

工具执行路径是特例：`recall` 失败时**不返回 Err**，而是返回正常的工具结果告知模型"检索暂时不可用"，让生成继续。把可选增强能力的故障冒泡成生成失败是不可接受的。

索引侧由 reconcile 设计天然保证——保存路径不知道索引的存在。

### 8.2 日志

遵循 `openspec/specs/unified-log-management/spec.md`。

| 事件 | 级别 | 内容 |
| --- | --- | --- |
| 索引批次完成 | `info` | 批大小、耗时、成功/失败条数、模型 id |
| 索引失败 | `warn` | 错误类别、attempt_count |
| 检索执行 | `debug` | scope 哈希、候选集大小、两路命中数、耗时 |
| 降级 | `warn` | 降级原因类别 |

**绝不落盘**：记忆内容、query 原文、API key、provider 响应体。query 只记长度与哈希——它可能包含用户敏感信息，而哈希足以关联同一查询的多次执行。

## 9. 测试策略

1. **纯函数单测**：RRF 融合（已知两个排名 → 期望顺序）、余弦（正交/相同/反向）、f32 BLOB 序列化往返、content_hash 稳定性、FTS query 转义（含 `"` `*` `-` `OR` 的 query 不被当查询语法解析）、失败分类映射（401→auth，429→rate_limit，超时→network）
2. **仓储集成测**：沿用 `memory_repository.rs:170-200` 的 `TempDirectory` + `NativeDatabase` fixture——reconcile 三类待办各自正确（新增 / hash 失效 / 孤儿清理）、scope 隔离不串、模型不匹配的行不进向量召回、FTS trigger 随增删改生效
3. **应用服务测**（fake `EmbeddingPort` + fake `EmbeddingEndpointPort`）：embedding 失败 → `degraded: keyword_only`；关键词路失败 → `degraded: vector_only`；全 pending → 关键词仍有结果；未配置 → 工具不注册；attempt 超限 → failed 且停止重试；auth 失败 → 立即 failed；`remove` 调用失败后 reconcile 仍能清掉孤儿
4. **迁移测**：沿用 `migration_fixture_tests.rs` 模式——老库升级到 42 不丢数据、重复执行幂等
5. **前端 vitest**：配置区块状态渲染、service 调用、web-agent-client 契约对等
6. **E2E（1 条）**：配置 embedding → 触发索引 → 状态由 pending 变 indexed，走 mock adapter，不打真实 API

## 10. 验收

按 AGENTS.md 全部跑通：

```
npm run lint
npm run test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
openspec validate --specs --strict
```

新功能需先在 `openspec/changes/` 起 proposal 并通过 `openspec validate --specs --strict` 再动代码。

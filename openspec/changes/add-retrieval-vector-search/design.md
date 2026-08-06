## Context

本设计整理自 `docs/superpowers/specs/2026-08-05-onepiece-vector-search-design.md`（OnePiece 向量检索 · 第 1 期设计），内容原样搬入本变更目录，确保归档后自包含、不依赖仓库中其他文档。以下四节分别对应原设计文档的 §3（已确认的关键决策）、§4.2（数据模型）、§4.3（跨上下文依赖方向）、§6.1（降级矩阵）。

## 已确认的关键决策

| 决策点 | 选择 | 理由 |
| --- | --- | --- |
| embedding 来源 | `EmbeddingPort` 抽象 + 远程 openai-compatible 实现 | 复用现有 provider catalog 与凭据体系，无新增重量级依赖；本地模型日后可插 |
| 向量存储 | SQLite BLOB + Rust 暴力余弦 | 单 agent 百~千条量级，扫描耗时远低于一次 embedding 网络往返；零跨平台构建风险 |
| 索引时机 | 后台异步补齐 | 保存路径不得因网络失败而失败 |
| 排序 | 向量 + FTS5 双路召回，RRF 融合 | 专有名词/路径/错误码这类字面 query 召回明显更好；embedding 不可用时天然降级 |
| 配置入口 | OnePiece 配置页新增"检索"区块，复用已保存 Profile | 不必新建一套完整的 Profile CRUD 与凭据管理 |
| 上下文归属 | 新建 `retrieval` peer context | 第 2/3 期要索引 `sessions` 与 `workspaces` 拥有的数据，内核留在 `agent_runtime` 会形成反向依赖 |

新增 peer context 需要一次架构决策并更新 `openspec/project.md:48-58` 的上下文表。

## 数据模型（迁移 43 `retrieval-vector-index`）

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

## 跨上下文依赖方向

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

## 降级矩阵

| 情况 | 行为 |
| --- | --- |
| 未配置 embedding | `recall` 工具不注册；现有 recency 注入照常工作 |
| 已配置但 query embedding 失败 | 只走关键词路，结果标 `degraded: keyword_only`，不报错 |
| 关键词路自身失败（FTS 语法或 IO 错误） | 只走向量路，结果标 `degraded: vector_only`，不报错 |
| 大量 pending 未索引完 | 关键词路照常——FTS 由 trigger 实时维护，不依赖 worker |
| 换了 embedding 模型 | 向量路只覆盖已收敛的行，其余走关键词，后台逐步补齐 |
| 两路都空 | 返回空列表，不是错误 |

FTS 独立于 embedding 可用，因此"未配置 embedding 就不注册工具"表面上浪费了关键词能力。这是刻意的：注册一个语义召回能力为零的 `recall` 会误导模型。**未配置 = 不注册；已配置但临时故障 = 降级关键词**。

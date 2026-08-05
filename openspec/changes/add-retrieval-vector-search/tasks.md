## 1. 实现任务

来源：`docs/superpowers/plans/2026-08-05-onepiece-vector-search-phase-1.md`（第 1 期实现计划）。本提案（Task 1）已完成；以下为 Task 2–17，按计划中的顺序与依赖关系执行，逐任务提交。

- [ ] 2. **`retrieval` 上下文骨架与领域类型** —— 建立 `contexts/retrieval` 目录骨架，定义 `SourceKind`/`IndexState`/`FailureCategory`/`RetrievalDocument`/`document_id`/`content_hash` 等领域类型与 `RetrievalError`。
- [ ] 3. **纯算法（余弦 / f32 BLOB 编解码 / RRF / FTS 转义）** —— 实现无 I/O 的算法内核：`encode_embedding`/`decode_embedding`、`cosine_similarity`、`fuse_with_rrf`、`escape_fts_query`，以及 `RetrievalScope`/`RetrievalQuery`/`ScoredHit` 等查询类型。
- [ ] 4. **迁移 42 与 schema** —— 新增 SQLite 迁移 `42 retrieval-vector-index`：`retrieval_documents` 表、FTS5 影子表（trigram）及三个同步 trigger、`retrieval_configuration` 单例配置表。
- [ ] 5. **文档仓储** —— 实现 `RetrievalDocumentRepository` trait 与 `SqliteRetrievalDocumentRepository`：upsert、按 scope/模型查候选、记录失败、索引状态统计、批量重新入队。
- [ ] 6. **配置仓储** —— 实现检索全局配置（来源 Profile、embedding 模型）的 `RetrievalConfigurationRepository` 读写。
- [ ] 7. **索引服务的差集协调** —— 实现 reconcile：新增记忆建 `pending` 行、内容哈希失效则重置 `pending`、源已删除的孤儿索引行清理；定义消费侧 `IndexSourcePort`。
- [ ] 8. **索引 worker 的批处理、失败分类与重试** —— 每批最多 32 条、串行调用 `EmbeddingPort`；`auth`/`invalid_request` 直接标 `failed` 不重试，`network`/`rate_limit` 按 1s/4s/15s/60s/300s 退避重试；内容超 8000 字符截断后再 embedding。可调常量集中定义。
- [ ] 9. **检索服务与降级** —— 实现向量路 + 关键词路双路召回、RRF 融合、回查源表取权威内容；embedding 失败降级 `keyword_only`，FTS 失败降级 `vector_only`，两路皆空返回空列表且不报错。
- [ ] 10. **openai-compatible embedding 适配器** —— 实现 `EmbeddingPort` 的 HTTP 适配器 `HttpEmbeddingAdapter`，并在 `application/ports.rs` 定义消费侧契约 `EmbeddingEndpointPort`。
- [ ] 11. **`agent_runtime` 侧的两条跨上下文契约** —— 新增 `agent_runtime::api::resolve_embedding_endpoint` 与 `list_embedding_models`；把 `is_chat_model` 的判定逻辑与 embedding 类模型过滤改为共享同一份模型类别派生，避免关键词表漂移。
- [ ] 12. **`retrieval::api`、bootstrap 装配与后台 worker** —— 实现 `retrieval` 唯一跨上下文出口（index/remove/search/配置/状态/重建），在 `bootstrap/retrieval.rs` 装配依赖并注册启动时全量 reconcile 与定时兜底轮询（默认 5 分钟）。
- [ ] 13. **`recall` 工具** —— 新增模型可调用的 `recall` 工具定义（`AutoApprove` 风险层级，scope 不进 schema），在 `resolve_tool_catalog` 中按是否已配置 embedding 条件性注入（含 plan mode 分支），在 `execute_tool_call` 中路由执行。
- [ ] 14. **记忆变更时的索引挂钩（删除撤销 + 保存唤醒）** —— 删除记忆后调用 `retrieval::api::remove` 撤销索引；`execute_remember` 保存成功后发送 worker 唤醒信号（不写库、不等待、失败无害）。
- [ ] 15. **Tauri commands 与前端服务边界** —— 新增 5 个检索相关 Tauri command 并注册；`agent-service.ts` 新增对应方法，`tauri-agent-client.ts` 与 `web-agent-client.ts` 同步实现；`types/agent.ts` 新增相关类型。
- [ ] 16. **配置区块 UI** —— 新建 `onepiece-retrieval-section.tsx`（来源 Profile 选择器、embedding 模型选择器、索引状态展示、重建索引动作），挂载进 `onepiece-configuration-panel.tsx`。
- [ ] 17. **E2E 与全量验收** —— 编写 1 条 E2E（配置 embedding 源 → 触发索引 → 状态由 `pending` 变 `indexed`，走 mock adapter，不打真实 API），并跑通 `npm run lint`、`npm run test`、`npm run build`、`cargo test`、`cargo check`、`openspec validate --specs --strict` 全量验收命令。

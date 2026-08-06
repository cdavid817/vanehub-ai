## 1. 实现任务

来源：`docs/superpowers/plans/2026-08-05-onepiece-vector-search-phase-1.md`（第 1 期实现计划）。本提案（Task 1）已完成；以下为 Task 2–17，按计划中的顺序与依赖关系执行，逐任务提交。

- [x] 2. **`retrieval` 上下文骨架与领域类型** —— 建立 `contexts/retrieval` 目录骨架，定义 `SourceKind`/`IndexState`/`FailureCategory`/`RetrievalDocument`/`document_id`/`content_hash` 等领域类型与 `RetrievalError`。
- [x] 3. **纯算法（余弦 / f32 BLOB 编解码 / RRF / FTS 转义）** —— 实现无 I/O 的算法内核：`encode_embedding`/`decode_embedding`、`cosine_similarity`、`fuse_with_rrf`、`escape_fts_query`，以及 `RetrievalScope`/`RetrievalQuery`/`ScoredHit` 等查询类型。
- [x] 4. **迁移 42 与 schema** —— 新增 SQLite 迁移 `42 retrieval-vector-index`：`retrieval_documents` 表、FTS5 影子表（trigram）及三个同步 trigger、`retrieval_configuration` 单例配置表。
- [x] 5. **文档仓储** —— 实现 `RetrievalDocumentRepository` trait 与 `SqliteRetrievalDocumentRepository`：upsert、按 scope/模型查候选、记录失败、索引状态统计、批量重新入队。
- [x] 6. **配置仓储** —— 实现检索全局配置（来源 Profile、embedding 模型）的 `RetrievalConfigurationRepository` 读写。
- [x] 7. **索引服务的差集协调** —— 实现 reconcile：新增记忆建 `pending` 行、内容哈希失效则重置 `pending`、源已删除的孤儿索引行清理；定义消费侧 `IndexSourcePort`。
- [x] 8. **索引 worker 的批处理、失败分类与重试** —— 每批最多 32 条、串行调用 `EmbeddingPort`；`auth`/`invalid_request` 直接标 `failed` 不重试，`network`/`rate_limit` 按 1s/4s/15s/60s/300s 退避重试；内容超 8000 字符截断后再 embedding。可调常量集中定义。
- [x] 9. **检索服务与降级** —— 实现向量路 + 关键词路双路召回、RRF 融合、回查源表取权威内容；embedding 失败降级 `keyword_only`，FTS 失败降级 `vector_only`，两路皆空返回空列表且不报错。
- [x] 10. **openai-compatible embedding 适配器** —— 实现 `EmbeddingPort` 的 HTTP 适配器 `HttpEmbeddingAdapter`，并在 `application/ports.rs` 定义消费侧契约 `EmbeddingEndpointPort`。
- [x] 11. **`agent_runtime` 侧的两条跨上下文契约** —— 新增 `agent_runtime::api::resolve_embedding_endpoint` 与 `list_embedding_models`；把 `is_chat_model` 的判定逻辑与 embedding 类模型过滤改为共享同一份模型类别派生，避免关键词表漂移。
- [x] 12. **`retrieval::api`、bootstrap 装配与后台 worker** —— 实现 `retrieval` 唯一跨上下文出口（index/remove/search/配置/状态/重建），在 `bootstrap/retrieval.rs` 装配依赖并注册启动时全量 reconcile 与定时兜底轮询（默认 5 分钟）。
- [x] 13. **`recall` 工具** —— 新增模型可调用的 `recall` 工具定义（`AutoApprove` 风险层级，scope 不进 schema），在 `resolve_tool_catalog` 中按是否已配置 embedding 条件性注入（含 plan mode 分支），在 `execute_tool_call` 中路由执行。
- [x] 14. **记忆变更时的索引挂钩（删除撤销 + 保存唤醒）** —— 删除记忆后调用 `retrieval::api::remove` 撤销索引；`execute_remember` 保存成功后发送 worker 唤醒信号（不写库、不等待、失败无害）。
- [x] 15. **Tauri commands 与前端服务边界** —— 新增 5 个检索相关 Tauri command 并注册；`agent-service.ts` 新增对应方法，`tauri-agent-client.ts` 与 `web-agent-client.ts` 同步实现；`types/agent.ts` 新增相关类型。
- [x] 16. **配置区块 UI** —— 新建 `onepiece-retrieval-section.tsx`（来源 Profile 选择器、embedding 模型选择器、索引状态展示、重建索引动作），挂载进 `onepiece-configuration-panel.tsx`。
- [x] 17. **E2E 与全量验收** —— 编写 1 条 E2E（配置 embedding 源 → 触发索引 → 状态由 `pending` 变 `indexed`，走 mock adapter，不打真实 API），并跑通 `npm run lint`、`npm run test`、`npm run build`、`cargo test`、`cargo check`、`openspec validate --specs --strict` 全量验收命令。

## 2. 验收记录（Task 17，2026-08-06）

### E2E：`tests/e2e/onepiece-retrieval.spec.ts`

新增 1 条 E2E，走 Web/mock adapter，不打真实 API：新增一个 openai-compatible 的 OnePiece 配置 → 在检索区块选择该配置与一个 embedding 模型并保存 → 点击"重建索引"并确认 → 断言索引状态从 mock 种子值（已索引 12 / 待索引 3 / 失败 2）变为（已索引 0 / 待索引 17 / 失败 0）。

`env -u all_proxy -u ALL_PROXY npx playwright test tests/e2e/onepiece-retrieval.spec.ts` → **1 passed**（15.6s）。

**已知限制（如实记录，不用更弱的断言掩盖）：** mock adapter（`src/services/web-agent-client.ts`）的 `rebuildRetrievalIndex` 只同步实现了"已索引/失败 → 待索引"的重新入队；全文件范围内全局的 `webRetrievalIndexStatus`（索引状态与重建同配置一样是全局的，不按 agent 分组）只被这一个函数改写，没有任何定时器或模拟 worker 把 `pending` 行推进为 `indexed`。这与真实后端的契约形状一致——`contexts/retrieval/api.rs` 的 `rebuild` 本身也只同步调用 `requeue_all`，真正把 `pending` 变成 `indexed` 的是异步后台 worker（Task 8/12），mock 没有对应的模拟实现，组件级单测（`onepiece-retrieval-section.test.tsx` 的 "requeues everything when rebuild is confirmed"）也是同样用 `pending` 增大来断言这次重建，独立印证了这一点。因此本 E2E 只能覆盖"配置 embedding 源 → 触发索引 → 已索引/失败行被重新入队为 pending"这一半路径；`pending → indexed` 的转换只发生在真实 Rust 后端由异步 worker 驱动，在不打真实 embedding 服务的前提下无法通过这个 mock 观测到。这是 mock 现有状态模型的诚实边界，不是本任务遗留的缺陷，也未对 mock 做任何投机性修改去"造出"这段行为。

### 全量验收命令（Step 3，共 8 条，逐条实际运行并观察输出）

| # | 命令 | 结果 |
|---|------|------|
| 1 | `npm run lint` | 通过：exit 0，`eslint .` 无输出（零告警/错误）|
| 2 | `npm run test` | 通过：131 个测试文件、539 个测试全部通过 |
| 3 | `npm run build` | 通过：`tsc && vite build && node scripts/check-frontend-chunks.mjs` 全部成功；"Verified 16 lazy frontend chunks; main static closure 105.7 KiB gzip" |
| 4 | `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check` | 通过：exit 0，无格式漂移 |
| 5 | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 通过：exit 0，零告警 |
| 6 | `cargo test --manifest-path src-tauri/Cargo.toml` | 通过：主库 1358 passed / 0 failed / 9 ignored（ignored 均为其他测试按需 spawn 的 fixture 用例，非跳过）；另有 `architecture`（12 passed）、`mcp_fixture_contracts`（3 passed）、`mcp_relay_provider_invocations`（3 passed）三个集成测试文件，全部 0 failed |
| 7 | `cargo check --manifest-path src-tauri/Cargo.toml` | 通过：exit 0 |
| 8 | `openspec validate --specs --strict` | 通过：85 passed / 0 failed |

已知偶发的 `contexts::tooling::mcp::infrastructure::relay` socket 测试**本次未出现**——`cargo test` 完整输出中该模块下全部测试（`relay`/`relay_jsonrpc`/`relay_legacy_sse`/`relay_stdio`/`relay_streamable_http` 等）与集成测试 `mcp_relay_provider_invocations.rs` 均为 `ok`，未触发已记录的既存 flake，因此无需按"确认是已知 flake"的流程重跑。

以上 8 条命令执行前后，`git status` 均确认 `src-tauri/gen/schemas/*.json` 未发生变化，也未修改任何生产代码。

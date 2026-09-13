# 规范与实现验证记录

日期：2026-09-09  
基线：`75f23d81406bdc1fd4d6b4d5a41fbf90e395c4fc`（worktree `feat/unified-memory`，直接自 `origin/main` 建立）。  
工具：OpenSpec CLI `@fission-ai/openspec@1.8.0`；Rust stable 1.97；Node/npm 按仓库 lockfile。  
状态：规范已完成；产品代码已实施；本记录只写实际执行过的检查。未执行的项明确标为 NOT RUN。

## 规范校验

| 检查 | 结果 |
|---|---|
| `validate unify-memory-read-scope --strict --no-interactive` | PASS，"Change 'unify-memory-read-scope' is valid" |
| `validate --specs --strict --no-interactive` | PASS，147/147 |
| 三个活动变更的 overlap | `strengthen-governed-cross-session-memory/tasks.md` 顶部登记本变更交付了其 2.3、4.4、5.1–5.3 的读取子范围；未勾选其任务，未改动其 specs。`harden-onepiece-generation-runtime`、`refactor-zh-cn-documentation-system` 无文件冲突 |
| `openspec/changes/archive/` | 未修改 |

## 仓库门禁（Linux，本机）

| 命令 | 结果 |
|---|---|
| `npm run lint:ci` | PASS |
| `npm run test` | 首轮 499/501 文件通过，2 个失败均为本变更未同步的测试期望（contract 测试把 `State<>` 参数按名字而非类型剔除；Web mock 索引状态多了 `keywordOnly`），修正后两文件 88/88 通过；全量复跑见下一行 |
| `npm run test`（修正后全量复跑） | PASS，501/501 文件，3098/3098 用例 |
| `npm run build` | PASS（16 lazy chunks，main closure 180.8 KiB gzip） |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASS |
| `cargo check --workspace` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `npm run native:panic:check` | PASS |
| `cargo test --workspace` | PASS，分段执行（见下） |
| `npm run contracts:check` | PASS，18/18 |
| `npm run docs:check` | PASS |
| `npm run architecture:check` | PASS（node 12/12、check.mjs、lint:ci、tsc、四个 fitness 测试 63/9/11/8） |
| `npm run desktop:unit:test` | PASS，72/72 |
| `npx playwright test tests/e2e/personalization-settings.spec.ts tests/e2e/session-personalization-mode.spec.ts` | 首轮 16/17（`keeps each session's mode when switching between them` 与并行 cargo 构建同时运行时超时），单独复跑该 spec 6/6 通过 |
| `npx playwright test`（全量，`PLAYWRIGHT_PORT=5199`，因 5174 被另一 worktree 会话的 dev server 占用；按 8 组顺序执行覆盖全部 61 个 spec） | PASS，278/278（40+22+36+29+35+36+41+39） |
| `npm run test:desktop:build` | PASS（`tauri build --debug --features desktop-e2e`，Linux） |
| `npm run test:desktop:smoke`（Linux，三次运行） | FAILED：25 个 spec 通过 23；`ui-settings.e2e.mjs` 两例（MCP 卡片删除按钮、Prompt Hook 卡片删除按钮 `matched 0`）三次均复现；`ui-notifications.e2e.mjs` 清空通知一例两次失败、补 PATH 后通过（flaky）。涉及文件与页面相对 `origin/main` 无差异，与记忆读取无关，未进一步排查；证据 `test-results/desktop/2026-09-09T17-20-00-418Z-88da8378` 等 |
| `npm run test:desktop:*` 各层（Linux；`PATH` 需含 `~/.npm-global/bin`，否则依赖真实 CLI 的层因 `opencode` 不在 PATH 而失败） | settings-persistence 2/2 PASSED；dialogs PASSED；session-workspace PASSED（1 例重试后通过）；session-deletion PASSED；scheduled-tasks PASSED；skills PASSED；local-media PASSED；agent-mcp PASSED；cli-terminal PASSED；cli-management 2/2 PASSED；ui-ratios PASSED；feishu-im 4/4 PASSED（补 PATH 前 3 例因 CLI 缺失失败）；session-shell PASSED（补 PATH 前失败）；loop FAILED：2 个 spec 中 1 个失败（向导“编辑/检查并启动”按钮 20 s 内不可点击，重试 2 次均复现），Loop Engineering 页面与本变更无关且相对 `origin/main` 无差异 |
| `npm run test:desktop` 未运行的层 | agent-evaluation、multi-agent-requirement/longrun、feishu-live、external-provider、core-smoke（均为需要真实模型或外部服务的可选层） |
| Windows / macOS Desktop Smoke | NOT RUN（无对应平台 runner） |
| MR-32 pool=10000（`cargo test -- --ignored a_ten_thousand_record_pool`，TMPDIR=/dev/shm） | PASS，见验收矩阵 MR-32；早先经 owner `create` 逐条播种的尝试在约 6,600 条后被内存守护终止，残留目录 `/dev/shm/personalization-compat-read-scope-scale-10000-*` 需手动清理（本会话无删除权限） |

`cargo test --workspace` 在本机 16 GB 内存、多 worktree 并行编译的条件下会被 OOM 杀死，因此先 `cargo test --workspace -j 1 --no-run` 编译全部测试目标，再按目标分段运行，覆盖与一次性运行完全相同的集合：

| 目标 | 结果 |
|---|---|
| `vanehub-ai` lib：`contexts::personalization` `contexts::retrieval` `contexts::agent_runtime` `bootstrap::retrieval` `bootstrap::personalization_bridge` | 2067 passed，2 ignored |
| `vanehub-ai` lib：`contexts::tooling` | 1526 passed |
| `vanehub-ai` lib：其余全部（`--skip` 上述前缀） | 3267 passed，12 ignored |
| `vanehub-ai` lib：加入快照日志行后复跑 `contexts::agent_runtime` `bootstrap::` `commands::` | 1769 passed，1 ignored |
| `--test architecture` | 63 passed |
| `--test evidence_bridge_architecture` / `log_repair_boundaries` / `mcp_fixture_contracts` / `mcp_relay_provider_invocations` / `remote_workspace_ssh` / `session_log_index_architecture` / `session_shell_architecture` | 12 / 9 / 3 / 3 / 15 / 11 / 8 passed |
| `vanehub-ai` bins、doctests | 0 tests（无） |
| `vanehub-permission-hook` | 25 passed |
| 新增 Context Engine memory source 测试后复跑 `context_sources` + `context_engine` | 8 passed |
| 新增端到端 generation 用例与 reconcile 中断用例后复跑 `bootstrap::memory_read_generation_tests` + `retrieval::application::indexing_service` | 21 passed |

lib 中 14 个 ignored 用例均为仓库既有的手动/环境相关用例，加上本变更新增的 `a_ten_thousand_record_pool_enumerates_completely_within_budget`（见 MR-32）。

## 验收矩阵对应

真实临时 v2 Markdown 目录 + SQLite/FTS5 投影 + 固定 embedding/provider fake 的用例位于：`contexts/personalization/api/read_scope_tests.rs`（API 级四入口）、`application/read_memory_tests.rs`（context/handle 域规则）、`infrastructure/memory_eligibility_tests.rs`（SQL 资格与完整关系）、`contexts/retrieval/application/search_service.rs` 与 `infrastructure/sqlite_repository.rs`（授权先于 top-k、单路降级、restricted 队列）、`retrieval/application/indexing_service.rs`（外发守卫）、`agent_runtime/infrastructure/memory_surfaced.rs`、`api_process_adapter/tests.rs`（recall 双门禁、selector 按 ID）、`infrastructure/context_sources.rs`（memory source 无 context 不运行）、`bootstrap/memory_read_generation_tests.rs`（真实栈单 generation 端到端，见下）、`bootstrap/personalization_bridge_tests.rs`、前端 `web-personalization-client.test.ts` 与 `runtime-preview-section.test.tsx`。

| ID | 结果 | 证据 |
|---|---|---|
| MR-01 / 02 / 03 / 05 | PASS | 端到端 `a_standard_generation_reads_exactly_its_scope_on_every_surface_over_the_real_stack`；API 级 `a_standard_session_reads_global_and_its_workspace_by_exact_audience_on_every_surface`、`disabling_global_access_or_project_only_keeps_the_workspace_and_drops_global_everywhere`、`an_absent_workspace_reads_global_only_while_an_unresolved_one_reads_nothing` |
| MR-04 | PASS | 端到端 `a_temporary_generation_completes_without_touching_any_memory_surface_over_the_real_stack`（无 selector 调用、无 recall、无 query embedding、manifest 记 memory source unavailable、生成完成）；`a_temporary_session_is_refused_by_every_governed_read_before_any_record_is_touched`、`a_temporary_session_keeps_custom_instructions_and_loses_every_memory_surface`、`recall_without_a_read_context_fails_closed_without_searching`、`the_memory_source_stays_idle_without_a_permitting_read_context_and_searches_with_one` |
| MR-06 | PASS | `a_selected_audience_admits_the_exact_stable_id_and_never_a_prefix_case_or_wildcard_variant`、`audience_membership_is_exact_under_sql_as_well_as_in_the_domain`、Web `admits a selected audience only to the Agents it names` |
| MR-07 | PASS | `a_projected_row_this_build_cannot_classify_is_excluded_rather_than_guessed_eligible`（`invalid_record`） |
| MR-08 | PASS（既有身份规则） | bridge 经 `WorkspaceIdentityRequest::from_session_workspace`（worktree > remote uri > project > legacy folder）；未新增用例 |
| MR-09 / 10 | PASS | `a_context_not_minted_by_this_host_or_altered_after_minting_is_refused`、`recall_ignores_scope_properties_the_model_invents_because_the_pool_is_shared`、`another_seat_in_the_same_session_does_not_inherit_the_suppression` |
| MR-11 | PASS | `the_recall_relation_is_complete_past_the_two_hundred_ref_injection_page`、`the_complete_relation_pages_every_eligible_row_and_never_the_injection_bound` |
| MR-12 | PASS | `unauthorized_rows_are_filtered_before_top_k_so_they_cannot_crowd_out_a_valid_hit`、`authorized_candidates_filter_both_paths_before_the_limit_...` |
| MR-13 | PASS | `query_embedding_failure_degrades_to_keyword_only_instead_of_erroring`、`keyword_path_failure_degrades_to_vector_only_instead_of_erroring`、`recall_returns_a_successful_result_when_retrieval_fails_so_generation_continues` |
| MR-14 / 31 | PASS | `an_incomplete_authority_set_is_refused_rather_than_searched`、`a_relation_larger_than_its_budget_is_reported_incomplete_rather_than_cut`、query-local 临时表用例 |
| MR-15 / 16 / 18 | PASS | `a_pinned_handle_is_dropped_when_the_record_moves_and_never_replaced_by_the_newer_version`、`an_external_edit_that_keeps_the_revision_is_detected_by_the_hash_and_dropped`、`an_archived_record_is_dropped_at_delivery_even_when_its_handle_is_otherwise_intact` |
| MR-17 | PASS（结构） | 快照在 Context Engine 之前冻结一次并贯穿 generation（`execute_with_resolved_personalization` 只接收已解析快照）；无跨 generation 的策略切换用例 |
| MR-19 | PASS | `two_records_with_one_name_are_told_apart_by_id_and_only_the_pinned_one_is_delivered`、`a_selected_name_the_selector_was_never_offered_reaches_no_body` |
| MR-20 | PASS | `memory_surfaced.rs` 三个用例（session+Agent/seat 分区、revision/hash 变化重新合格）；端到端 `a_session_in_another_workspace_never_reuses_the_previous_workspace_authorization` |
| MR-21 | PASS | memory source 无 context 不运行、候选按 `memory:<id>@<revision>` 标识；API 运行时的模型轮次只来自会话历史，宿主不重注入旧 memory 候选（Context Engine 每代在当前 context 下重新检索，manifest 只记录不回放）；跨工作区端到端用例证明前一会话的授权、surfaced 标记与 relation 均不复用 |
| MR-22 / 23 | PASS | `index_maintenance_lists_every_active_record_and_marks_scoped_or_restricted_bodies_keyword_only`、`a_restricted_record_is_keyword_searchable_but_never_claimed_for_embedding`、`a_public_row_that_becomes_restricted_drops_its_vector_and_retires_to_keyword_only`、`a_body_the_authoritative_record_no_longer_permits_is_retired_before_dispatch_not_embedded`、`reconcile_queues_a_restricted_record_as_keyword_only_and_notices_a_restriction_change` |
| MR-24 | PASS | `vector_candidates_exclude_rows_embedded_with_a_different_model`、`requeue_stale_model_leaves_rows_already_on_the_configured_model_alone`、`an_interrupted_reconcile_applies_nothing_and_the_next_run_recomputes_the_whole_diff` |
| MR-25 | PASS | owner 列表/管理 API 保持独立入口；Agent 读取只经 `GovernedMemoryReadService`（无 `Option<Context>` 入口） |
| MR-26 | PART | `a_context_frozen_under_one_maintenance_generation_is_refused_after_the_store_moves_on`；LKG 规则沿用既有快照实现，未新增用例 |
| MR-27 | PASS | 预览 `recallAvailability`（disabled/unsupported/unconfigured/unavailable/available）与空池：`tells an allowed empty pool apart from blocked reading`、`names why reading is blocked and reports the recall channel off with it` |
| MR-28 | PASS | `resolves under the chosen session's own id, mode and workspace`、`stops the mode and workspace being edited while a session is selected`；native 端 `preview_resolution` 以存储会话覆盖调用方字段 |
| MR-29 | PASS | Web `applies the Agent layer's read switch on top of the global one`、`drops global memories in a project-only session`、`reports an allowed empty pool as allowed rather than as disabled` |
| MR-30 | PASS | `the_scope_fingerprint_carries_no_identifiers_a_log_should_not_hold`、`a_digest_carries_no_memory_text`、`unified_diagnostic_excludes_prompt_source_memory_credentials_and_payloads`；快照日志只含 contract 版本、scope 指纹与两个计数 |
| MR-32 | PASS | 播种改为直接写 v2 文件 + 一次 `reconcile_memories`（owner `create` 逐条 fsync/发布是单条记忆的正确行为，不是万条播种的工具）。20 次采样：pool=1000 relation P50 28.8 ms / P95 37.7 ms，pinned read（20 条）P50 2.9 ms / P95 4.8 ms，reconcile 12.8 s（磁盘）；pool=10000 relation P50 247 ms / P95 277 ms，pinned read P50 2.5 ms / P95 3.1 ms，reconcile 9.4 s（tmpfs）；两者正文加载均为 20 条，进程峰值 RSS 约 100 MiB。relation 随池线性（约 25 µs/行，完整枚举是设计要求），正文读取与池大小无关。query plan：`SEARCH personalization_memory_projection USING INDEX sqlite_autoindex_..._1 (memory_id>?)` + `CORRELATED SCALAR SUBQUERY`（audience 的 `json_each` 精确匹配，逐行）；无 `USE TEMP B-TREE FOR ORDER BY`（已断言）。检索侧 `the_authorized_keyword_query_probes_the_relation_by_primary_key`：`SCAN f VIRTUAL TABLE INDEX 0:M1` → `SEARCH d USING INTEGER PRIMARY KEY` → `SEARCH a USING PRIMARY KEY (source_id=?)`，query-local 关系按主键探测、从不扫描（已断言）；bm25 排序的 temp b-tree 为 FTS 排序固有 |

`最小真实集成验收`：PASS。`bootstrap/memory_read_generation_tests.rs` 用真实 v2 目录、SQLite 投影与 FTS、真实 `GovernedPersonalizationAdapter`（无 sessions owner，工作区按 legacy folder 规则解析）、真实 `RetrievalApi`/`IndexingService`/`DeferredAgentRetrieval`、真实 Context Engine 与 `RuntimeAgentApiAdapter`，模型为脚本化本地 HTTP 端点，embedder 为可计数的固定向量。四条记录（G-all、W1-all、W2-all、G-selected-agent-b）全部命中查询，标准会话只在 selector 清单、索引页、选中正文、Context Engine manifest 与 recall 结果中看到 G-all 与 W1-all；临时会话零 memory 调用仍完成；第二个工作区的会话不复用第一个的授权。MR-11（>200）保留在 API 级用例（每条 owner 写入含 fsync，端到端播种 200 条过慢）。

## 兼容与边界

- 新增 migration 115 `retrieval-memory-egress-restriction`：`retrieval_documents.egress_restricted` 列，附加式，不重写 v2 文件与用户 scope/audience/policy。
- Tauri `preview_effective_personalization` 新增 `SessionsApi`/`RetrievalApi` 依赖；DTO 增加 `previewKind`、`memoryReadAllowed`、`readBlockReason`、`indexEntryCount`、`indexTruncated`、`recallAvailability`。Web mock 同步。
- 检索索引状态增加 `keywordOnly` 计数。
- restricted（scoped/selected audience）记忆首版固定 FTS-only；claim/retry/rebuild/模型切换统一排除，embed 前回源复核。
- CLI Agent 只承接 index 注入；`recall` 只在原生工具循环存在，预览以 `unsupported` 区分。
- 架构预算：`src/services` 28404；`agent_runtime/infrastructure` 74,581 / 40,966；`platform/database` 3,813；`eligible_authority` 登记于 deferred-transaction 允许清单。

## NOT RUN / 未完成

- `npm run test:desktop`：smoke 层两例与 loop 层一例与记忆无关的既有失败（相关文件相对 `origin/main` 无差异）；可选的真实模型/外部服务层未运行；Windows/macOS 平台验证。
- 跨 generation 策略切换用例（MR-17 仅结构验证）。
- 观察到的既有行为（非本变更范围）：API 运行时把 Context Engine 的 `<context-evidence>` 追加到 `effective_prompt`，但模型轮次来自会话历史，该证据文本不会进入 provider 请求；端到端用例因此以 manifest 的 source outcome 与 embedder 输入断言 memory source 的运行。
- 本地 `main` 未从 `origin/main` fast-forward（worktree 会话无法操作主检出）。

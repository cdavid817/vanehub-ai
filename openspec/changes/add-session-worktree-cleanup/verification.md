# 验证状态与执行记录

## 包生成时的状态（保留）

- 生成日期：2026-09-05。
- 代码证据基线：main@d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5。
- 包生成时未实现代码、未运行任何测试；下文「实施记录」才是实际执行结果。

## 实施记录（2026-09-05）

- 实施 checkout：worktree `.claude/worktrees/main-20260905`，分支 `worktree-main-20260905`，基于 `main@d6e1d6ff`（与 `origin/main` 同步）。所有改动仍是工作区未提交修改；没有 push、合并或归档。
- 平台：Linux 7.0.0-29-generic x86_64；git 2.43.0；rustc 1.97.1；node v22.23.2；npm 10.9.8；openspec CLI 1.9.0。
- 破坏性测试只作用于测试自建的临时 Git 仓库（`tempfile`）和临时 SQLite 数据库；没有触碰用户的真实项目、既有 worktree、生产会话或凭据。
- 实施前基线：`openspec validate add-session-worktree-cleanup --strict` 通过（修正 MODIFIED 块与链接后），`openspec validate --specs --strict` 通过。

### 门禁执行结果

| 类别 | 命令 | 结果 | 证据/限制 |
| --- | --- | --- | --- |
| Change 规范 | `openspec validate add-session-worktree-cleanup --strict` | PASSED | "Change 'add-session-worktree-cleanup' is valid" |
| 主规范 | `openspec validate --specs --strict` | PASSED | 147 passed, 0 failed |
| 前端 lint | `npm run lint:ci` | PASSED | `eslint . --max-warnings=0` 无输出 |
| 前端测试 | `npm run test` | PASSED | 476 files / 2938 tests |
| 前端覆盖率 | `npm run test:coverage` | PASSED | 476 files / 2938 tests，阈值通过 |
| 前端构建 | `npm run build` | PASSED | vite 5447 modules |
| 契约 | `npm run contracts:check` | PASSED | 33 个 sessions 命令，含 5 个 deletion 命令 |
| 架构 | `npm run architecture:check` | PASSED | 需上调 `src/services` 行数预算 27405→28063（理由已写入 `scripts/architecture/frontend-rules.mjs`） |
| 覆盖率策略/版本 | `npm run coverage:policy:test` / `npm run version:unit:test` | PASSED | 退出码 0 |
| 格式 | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASSED | 无差异 |
| Rust | `cargo check --workspace` | PASSED | 两个 crate 均通过 |
| Rust lint | `cargo clippy --workspace --all-targets -- -D warnings` | PASSED | 清理了未使用的 re-export、dead code、`byte_char_slices`、`type_complexity` |
| Panic gate | `npm run native:panic:check` | PASSED | 生产代码无 `unwrap/expect` |
| Rust test | `cargo test --workspace` | PASSED | lib 6677 passed / 13 ignored；集成测试全过。`tests/architecture.rs` 在命令重构后单独重跑：62 passed |
| Rust 架构 | `tests/architecture.rs` | PASSED | 命令文件改为只做传输映射（分支/阻塞逻辑移到 `commands/sessions/background.rs`）；原生子树预算按实测上调（agent_runtime/infrastructure 62914 / 生产 33901，platform/database 3648），理由写在测试文件 |
| Web UI | `PLAYWRIGHT_PORT=5199 npx playwright test` | PASSED | 251 passed（43.2 分钟），含 `tests/e2e/session-deletion.spec.ts` 5 例。端口 5199 是因为 5174 被另一会话占用 |
| Desktop unit | `npm run desktop:unit:test` | PASSED | 退出码 0 |
| Desktop native | `npm run test:desktop:build` + `test:desktop:smoke` + `test:desktop:dialogs` + `test:desktop:session-deletion`（Linux） | 见下表 | 仅当前平台；新增 `desktop-session-deletion` 层驱动真实对话框与真实 Git |
| 文档 | `npm run docs:check` / `npm run docs:links:check` | PASSED | "Documentation links, media, and boundary inventories verified." |
| 工作区差异 | `git diff --check` | PASSED | 退出码 0 |

### 桌面分层结果（按平台）

| 平台 | 层 | 结果 | 证据/限制 |
| --- | --- | --- | --- |
| Linux | desktop-smoke（本地全量 required 集，25 个 spec） | FAILED（23 passed / 2 failed，含重试） | 证据 `test-results/desktop/2026-09-05T13-06-06-108Z-32f9b5a5/`。失败 1：`domain-cli-tooling.e2e.mjs`「saves, reads back and resets the opencode parameter selection」，用 `VANEHUB_DESKTOP_SPEC` 单独重跑通过，判定为 25 个 worker 并行下的偶发。失败 2：`domain-skills.e2e.mjs`「takes a Skill tool revision through validate, trust, enable, quarantine and recover」，`set_skill_tool_trust` 返回 `storage`，重试后因状态已持久化变成 `'valid' !== 'pending'`；用 `git archive main` 导出的**未改动 main** 在本机同样构建并单独重跑，失败完全一致（`Error: storage` → `'valid' !== 'pending'`），因此是本机环境/既有缺陷，不是本变更引入；main 在 CI ubuntu runner 的 Desktop Smoke 为 success。启动、IPC、导航及其余 23 个 spec 通过 |
| Linux | desktop-dialogs | PASSED | 1 spec / 2 passing，证据 `test-results/desktop/2026-09-05T13-16-05-455Z-78056dc1/`。该层覆盖既有主路径对话框 |
| Linux | desktop-session-deletion（新增） | PASSED | 1 spec / 4 passing，连续两次运行通过，证据 `test-results/desktop/2026-09-05T15-24-19-854Z-fa3e3f92/` 与 `2026-09-05T15-27-36-798Z-061cc439/`。用例：默认保留且取消不改动磁盘；有未跟踪文件时拒绝清理并给出原因；显式选择后经 Git 移除目录与登记、分支与主 checkout 保留、会话消失；项目会话不提供清理且目录不受影响。固定 fixture `opencode`（`prepareManagedCliFixtures` + `VANEHUB_CLI_FIXTURE_ROOT`），临时仓库在 `os.tmpdir()` 下自建 |
| Linux | 其余层（cli-terminal、cli-management、session-workspace、session-shell、scheduled-tasks、settings-persistence、agent-mcp、local-media、skills、feishu-im 等） | NOT RUN | 本变更未改这些层覆盖的行为；未运行不代表通过 |
| Windows | 全部 | NOT RUN | 本机无 Windows runner；需 CI `Desktop Smoke` artifact |
| macOS | 全部 | NOT RUN | 本机无 macOS runner；需 CI `Desktop Smoke` artifact |

### 针对性测试（先于全量门禁执行）

| 命令 | 结果 |
| --- | --- |
| `cargo test --lib -- worktree_cleanup deletion advisory_lock instance_lease worktree_git_parsing worktree_ignored_scan managed_worktree_repository worktree_resource contracts migrations` | 139 passed |
| `npx vitest run src/main-layout/session-deletion src/services/web-session-deletion-client.test.ts src/main-layout/session-sidebar src/services/web-agent-client` | 通过（删除相关 17 + web client 7 + 侧栏/web-agent 92） |
| `PLAYWRIGHT_PORT=5199 npx playwright test tests/e2e/session-deletion.spec.ts` | 5 passed |

### 代码复审修正（2026-09-05，第二轮）

按性能、UI、逻辑三类自查后修正如下，修正后重跑：`cargo test --lib -- sessions workspaces bootstrap commands migrations`（1249 passed）、`tests/architecture.rs`（62 passed）、clippy `-D warnings`、fmt、`npm run test`（2939 passed）、`npm run lint:ci`、`tsc --noEmit`、`npm run architecture:check`、Playwright 删除 spec（5 passed）。

| 类别 | 问题 | 修正 |
| --- | --- | --- |
| 逻辑 | `SessionDeletionCoordinator::run` 中某个组的内部错误（journal/门禁存储失败）通过 `?` 中止整个操作，操作永远停在 `quiescing`，对话框因禁止关闭而永久锁死 | 逐组捕获错误：未开始移除的组记为 `failed`（`deletion_run_failed`）并释放 claim；已写入 `remove_started`/结果未知的组进入 `needs_attention` 并保留 claim；目录已确认移除的组进入 `finalize_pending`；其余组继续，操作必定 `finish`。门禁在错误路径上释放。新增 2 个测试 |
| 逻辑 | 引用扫描对已不存在的路径用字符串前缀判断，`/repo/wt2` 会被当成 `/repo/wt` 的引用（过度阻止清理） | 只在路径分量边界上判定包含；新增 `points_into` 单元测试 |
| 性能 | 执行链路上的 `begin_removal`、`remove_safely`、`observe`、历史来源核验、创建后确认都做**完整**探针（status 全树遍历 + ignored 清单 + `ls-files` 索引扫描），一次 remove-safe 约 4 次全量探针 | 探针端口新增 `probe_identity`（身份/登记/锚点，不读工作区内容），这些步骤改用它；预览与执行前的最终核验仍用全量探针 |
| 性能 | 预览时每个会话都读取整个 operations 列表来找创建证据，即使会话已有 managed 记录 | 先查 `managed_worktree_sessions`，只有无记录的普通会话才读 operations |
| 性能 | 引用扫描对每个会话的 3 个候选路径各做一次 `canonicalize` 系统调用，同一项目路径重复解析 | 每次扫描内按路径去重缓存 |
| 性能 | 启动时 `reconcile_pending_deletions` 同步运行在 bootstrap 路径上，若有中断操作会对每个目录做 Git 探针（最长 10s/次） | 移到后台线程；期间会话仍由 journal claim 拦住新工作 |
| 性能/UI | 前端轮询 effect 依赖整个 `state`，每次进度更新都清除并重建定时器；一旦有组删除成功，之后每次轮询都触发三组 query invalidation | effect 只按 operationId 键控；仅在已删除组数增加时 invalidation；轮询有 in-flight 保护；间隔 400ms→500ms |
| UI | 执行中若状态查询持续失败（操作丢失、adapter 断开），对话框既不能关闭也不会结束 | 连续 10 次查询失败后退出跟随、显示错误并允许关闭；journal 仍是真相。新增 hook 测试 |
| UI | `refresh` 在 `setState` updater 内触发副作用，StrictMode 下会重复发起预览 | 改为读取当前 state 后直接调用 |

未修正但已确认的设计取舍：含子模块（`.gitmodules`）、sparse checkout 或索引带 assume-unchanged/skip-worktree 标记的 worktree 一律只允许 keep；预览与执行之间出现新提交会因身份变化被拒绝并要求重新检查；`is_path_gated` 对每个祖先目录各做一次门禁查询（Shell 创建时约 5–10 次小查询）。

## 改动文件

### Rust（`src-tauri/`）

- platform：`platform/filesystem/advisory_lock.rs`（新，OS 级建议锁）、`platform/instance_lease.rs`（新，实例租约/存活判断）、`platform/git/mod.rs`（`execute_isolated`：剥离 `GIT_DIR` 等仓库选择变量、`--no-optional-locks`、关闭 fsmonitor）、`platform/process/mod.rs`（`env_remove`）、`platform/database/migrations/mod.rs`（112 `managed-worktree-resources`、113 `session-deletion-operations`）、`platform/filesystem/mod.rs`、`platform/mod.rs`。
- workspaces：`domain/worktree_resource.rs`（新：来源/出处/状态/身份与受守卫的状态迁移）、`domain/error.rs`、`application/worktree_cleanup_models.rs`（新：探针/移除/门禁/仓库端口与模型）、`application/worktree_cleanup_policy.rs`（新：纯策略 `evaluate_cleanup` 与 reason code）、`application/worktree_cleanup.rs`（新：意图登记、确认、历史来源核验、检查、门禁 claim、begin_removal 时二次身份核验、非 force 移除、观察、finalize）、`application/service.rs`（`plan_worktree` / `create_planned_worktree`）、`infrastructure/worktree_git_parsing.rs`（新：NUL/字节安全解析）、`infrastructure/worktree_ignored_scan.rs`（新：有界 ignored 清单与指纹）、`infrastructure/worktree_probe.rs`（新：Git 探针与唯一非 force `worktree remove`）、`infrastructure/managed_worktree_repository.rs`（新：SQLite 资源表、工作区使用门禁）、`infrastructure/worktree_cleanup_tests.rs`（新：真实临时仓库测试）、`api.rs`（清理转发、执行准入端口、创建链路接入意图/确认/needs_attention）。
- sessions：`application/deletion/{mod,models,ports,policy,coordinator,tests}.rs`（新：预览/执行/运行/重试/启动恢复协调器）、`application/service.rs`（执行准入检查、创建时绑定与准入）、`application/ports.rs`、`application/models.rs`、`infrastructure/deletion_journal.rs`（新：journal 表与事务）、`infrastructure/deletion_adapters.rs`（新：workspace/引用/预览存储/时钟/ID/owner 适配）、`infrastructure/deletion_runtime.rs`（新：quiesce 实现）、`infrastructure/deletion_journal_tests.rs`（新）、`infrastructure/creation_context.rs`、`infrastructure/runtime_support.rs`、`api.rs`（`delete` 改为经协调器的 keep-only）。
- agent_runtime：`infrastructure/tools/background_shell.rs`（`reap_session_and_wait`）、`api.rs`（`reap_background_commands_and_wait`）。
- commands：`commands/sessions/{preview,execute,get,list_pending,retry}_session_deletion.rs`（新）、`delete_session.rs`（异步；只有删除的是活动会话才发 active-session-changed）、`background.rs`（`spawn_deletion*`、`preview_deletion_off_thread`、`delete_session_off_thread`）、`events.rs`（`sessions-changed`）、`mod.rs`、`core_registry.rs`、`contracts.rs`。
- bootstrap：`runtime.rs`（数据库后获取 InstanceLease；绑定执行准入；恢复后 `reconcile_pending_deletions`）、`sessions.rs`、`workspaces.rs`、`mod.rs`。
- tests：`tests/architecture.rs`（三处原生预算按实测上调并写明理由）。

### 前端（`src/`）

- 类型/服务：`types/session-deletion.ts`、`services/session-deletion-service.ts`、`services/tauri-session-deletion-client.ts`、`services/web-session-deletion-{client,runner,simulation,state}.ts`（Web/mock 全部标记 simulated）、`services/web-session-deletion-client.test.ts`、`services/agent-service.ts`、`services/tauri-agent-client.ts`、`services/web-agent-client.ts`、`services/web-session-lifecycle-client.ts`（旧 `deleteSession` 走 claim 校验的 keep-only）。
- UI：`main-layout/session-deletion/{session-deletion-model.ts,use-session-deletion.ts,session-deletion-dialog.tsx,session-deletion-worktree-row.tsx,session-deletion-result.tsx}` 及三个测试文件；`main-layout/main-layout.tsx`（518 行）、`session-sidebar.tsx`（279 行，自带确认框移除）、`session-context-panel.tsx`（内联删除 modal 移除）、`use-main-layout-model.ts`（删除 mutation 移除）。所有新增生产文件 ≤300 行，未新增豁免。
- i18n：`i18n/locales/{zh-CN,zh-TW,en,ja,ko}.json` 各新增 115 个 `sessionDeletion.*` 键。
- E2E：`tests/e2e/session-deletion.spec.ts`（5 例，Web mock）；桌面层 `tests/desktop/specs-session-deletion/session-deletion.e2e.mjs` + `tests/desktop/wdio.session-deletion.conf.mjs`（4 例，真实 Git），注册于 `scripts/test-desktop.mjs`、`package.json`（`test:desktop:session-deletion`）、`scripts/desktop-orchestrator.node-test.mjs`、AGENTS.md 层清单；`tests/desktop/helpers/native-ui.mjs` 的 `assertNoFatalError` 附带错误详情。
- 架构规则：`scripts/architecture/frontend-rules.mjs`（`src/services` 预算 28063，理由见注释）。

### 文档

- `docs/user-guide/zh-CN/src/worktree.md`、`docs/user-guide/en/src/worktree.md`：删除会话与 worktree 清理章节（默认保留、分支保留、忽略文件确认、共享/来源不明阻止、失败恢复；Loop 规则不变）。
- `docs/developer-guide/zh-CN/src/persistence-ownership.md`、`docs/developer-guide/src/persistence-ownership.md`：`managed_worktrees`、`session_deletion_*` 表的归属与生命周期。

## 需求 → 自动化测试映射

证据等级：R = Rust 单元/集成（真实临时 Git 仓库或临时 SQLite）；F = 前端 Vitest；E = Playwright（Web mock，simulated）；D = 桌面 WDIO 层（真实客户端与真实 Git，仅 Linux）。未列出的 TC 见「未自动化项」。

| 需求 | 测试 |
| --- | --- |
| S-01（保留原场景） | R `sessions::application::tests`（既有）、`deletion/tests.rs::deleting_the_active_session_clears_the_selection_and_publishes_once`、`keep_deletes_sessions_after_quiescence_and_clears_the_active_session_only_when_included`（TC-005/006）；`worktree_cleanup_tests::keep_leaves_directory_registration_branch_and_record`（TC-007） |
| S-02 默认保留/选择 | F `session-deletion-model.test.ts`「default to keep for every worktree and never persist a destructive choice」「ignore a toggle on a worktree that only allows keep」「switch the confirm label…」；F `session-deletion-dialog.test.tsx`「shows the project note without any worktree option…」「defaults to keep, switches the confirm label…」；E「a project session confirms with no worktree option…」「a worktree session keeps its directory by default…」（TC-008/010/011/012） |
| S-03 取消与不可用 | E「…cancelling deletes nothing」（TC-013）；R `worktree_cleanup_policy` `GIT_UNAVAILABLE` 用例（TC-014）；F dialog「keeps a blocked worktree visible with its reasons and the option disabled」（TC-015） |
| S-04 忽略文件确认 | R `ignored_files_require_an_acknowledgement_bound_to_a_fingerprint_that_tracks_the_files`、`deletion/tests.rs::ignored_acknowledgements_are_rechecked_against_the_current_inventory`；E「an ignored inventory needs its own acknowledgement…」（TC-016/017/018） |
| S-05 进行中/失败/待完成 | F `use-session-deletion.test.ts`「executes with a stable request id, follows the operation, and refuses to close mid-flight」「retries a finalize-pending group without a new preview」；F dialog「blocks closing while executing and renders per-group results afterwards」（TC-019/020/021/022） |
| S-06 入口与批量 | E「batch deletion opens the same dialog, groups by worktree, and keeps failed targets selected」；F `session-sidebar` 既有测试（TC-023/024/025） |
| S-08 旧调用者 | R `the_legacy_keep_only_path_runs_the_same_coordinator_and_respects_claims`；F use-session-deletion「previews on request, defaults to keep, and never calls the legacy delete」；`npm run architecture:check` 禁止组件直接 invoke（TC-028/029） |
| S-09 多会话删除（MODIFIED） | E 批量用例；R `independent_groups_keep_their_own_outcomes_and_aggregate_to_partial`（TC-096..TC-099） |
| W-01 来源登记 | R `a_failed_creation_marks_the_intent_for_attention_instead_of_deleting_anything`、`legacy_sessions_are_verified_only_with_complete_evidence`、`worktree_resource.rs` 四个状态迁移测试（TC-030..033） |
| W-02 身份 | R `main_workspaces_plain_directories_nested_layouts_and_links_are_refused`、`a_replaced_root_never_matches_the_recorded_identity_and_is_not_removed`；`deletion/tests.rs::execute_never_accepts_a_removal_the_preview_did_not_allow`（TC-034..037） |
| W-03 分支/布局 | R `a_clean_verified_worktree_is_removed_without_force_and_the_branch_survives`、`detached_heads_locked_worktrees_and_in_progress_operations_block`（TC-038..041） |
| W-04 未保存内容 | R `uncommitted_content_of_every_kind_blocks_and_git_itself_refuses_a_non_forced_remove`、`worktree_git_parsing` 五个解析测试（rename/换行/引号/非 UTF-8/截断）（TC-042..045） |
| W-05 忽略清单 | R ignored 指纹测试、`worktree_ignored_scan` 测试（不读正文、有界、SECRET 不出现在样本）（TC-046..048） |
| W-06 引用 | R `preview_counts_external_references_and_never_treats_an_incomplete_scan_as_empty`、`a_reference_that_appeared_after_the_preview_blocks_removal`、`preview_classifies_project_remote_and_worktree_sessions_and_deduplicates_worktrees`（TC-049..052） |
| W-07 移除 | R `a_clean_verified_worktree_is_removed_without_force…`、`a_refused_removal_with_an_intact_directory_awaits_a_decision_and_frees_the_sessions`、`an_uncertain_removal_effect_parks_the_group_and_never_removes_again`、`a_timed_out_removal_that_is_observed_gone_counts_as_removed`（TC-053..056） |
| W-08 保留 | R `keep_leaves_directory_registration_branch_and_record`、`completing_a_group_deletes_the_rows_clears_a_matching_active_session_and_releases_claims`；`worktree_resource.rs::loop_and_external_origins_never_become_eligible`（TC-057..059） |
| O-01 预览/授权 | R `preview_refuses_empty_oversized_unknown_and_system_selections`、`execute_rejects_expired_previews_and_binds_requests_idempotently`、`changes_after_the_preview_are_caught_by_the_final_revalidation`；F web client「refuses system activity ids, empty selections and unknown sessions before touching anything」（TC-060..062） |
| O-02 journal | R `a_journal_failure_before_acceptance_starts_nothing`、`remove_safe_journals_before_git_and_deletes_sessions_only_after_confirmed_removal`、`deletion_journal_tests` 四例（TC-063..065） |
| O-03 静止 | R `a_session_that_will_not_quiesce_keeps_its_record_and_directory`；`background_shell.rs::reap_session_and_wait` 等待真实退出（TC-066..068） |
| O-04 门禁/多实例 | R `gates_are_exclusive_across_operations_and_paths_under_a_gated_root_are_refused`、`gates_are_exclusive_per_root_and_released_by_their_owner_only`、`a_gate_held_by_a_dead_instance_can_be_taken_over`、`a_gate_held_elsewhere_blocks_removal_without_stopping_anything`、`instance_lease`/`advisory_lock` 五例（TC-069..072） |
| O-05 最终核验 | R `changes_after_the_preview_are_caught_by_the_final_revalidation`、`an_identity_that_drifted_after_the_preview_is_refused_before_git_runs`、`a_reference_that_appeared_after_the_preview_blocks_removal`（TC-073..075） |
| O-06 事务 | R `git_success_followed_by_a_database_failure_leaves_finalize_pending_with_claims_held`、`completing_a_group_deletes_the_rows…`（TC-076..078） |
| O-07 幂等/重试 | R `execute_reuses_a_request_id_only_for_identical_content`、`retry_refuses_a_stale_revision_and_a_preview_over_different_targets`；F web client「returns the same operation for an identical request id and conflicts on different content」「reports partial batches per group and retries only the unfinished group…」「leaves a finalize-pending group with its claim and finishes it on a database-only retry」（TC-079..081） |
| O-08 分组 | R `preview_classifies…deduplicates_worktrees`、`independent_groups_keep_their_own_outcomes_and_aggregate_to_partial`（TC-082..084） |
| O-09 恢复 | R `recovery_finalizes_a_removal_it_can_observe_and_never_reruns_git`、`recovery_with_an_intact_directory_asks_for_a_new_decision`、`recovery_parks_ambiguous_or_offline_resources`、`recovery_of_an_operation_that_never_started_git_releases_the_sessions`、`recovery_leaves_an_operation_owned_by_a_live_instance_alone`（TC-085..090） |
| O-10 诊断/预算 | R 解析截断与 `ProbeBudget` 测试、ignored 样本不含 SECRET；reason code 为结构化常量（TC-091/092） |
| O-11 适配 | D `desktop-session-deletion` 4 例（真实对话框 + 真实 Git：目录与登记消失、分支保留、主 checkout 完好）；F web client「previews as simulated…」「simulates a removal, dedupes shared worktrees, and never claims a native effect」；R `worktree_cleanup_tests`（真实临时仓库：目录消失、登记消失、分支保留）；`web-http` 无 adapter 时显式错误由既有 conformance 测试覆盖（TC-093..095） |

### 未自动化项（不视为通过）

- TC-026（键盘/辅助技术）、TC-027（长路径/平台路径）：对话框复用 `ApplicationDialog`（焦点陷阱、Esc、autofocus）且行内有 aria 属性，但没有专门的可访问性/小窗口/深浅色自动化用例。
- TC-094 原生端到端：由新增的 `desktop-session-deletion` 层覆盖（Linux PASSED）；Windows/macOS 仍 NOT RUN。
- 三平台桌面结果只有 Linux 本机数据；Windows/macOS NOT RUN。

## 剩余限制与风险

- 历史（legacy）worktree 会话：只有当来源证据完整且与当前 Git 身份一致时才允许 remove-safe；大多数迁移前会话只有 keep。
- 派生 watcher（LSP/文件索引）视为可释放句柄，不作为阻止清理的业务引用。
- `archive_session` 命令仍无条件发 active-session-changed(None)，不在本变更范围内。
- `src/services`、`agent_runtime/infrastructure`、`platform/database` 行数预算按实测上调，理由已写入规则文件；未新增 ESLint 豁免。
- 未新增删除对话框的桌面 WDIO 层；跨重启恢复只有 Rust 级 `reconcile_pending` 测试，没有真实重启用例。
- 本机 `desktop-smoke` 的 skills spec 失败在未改动的 main 上可复现（见上表），需要在干净 runner 或 CI 上复核；本变更不修复它。
- 未 push、未合并、未归档。tasks.md 未勾选项：7.9（缺可访问性/小窗口/深浅色自动化证据）。

### 桌面层发现并修复的缺陷（2026-09-05，第三轮）

`desktop-session-deletion` 层首次运行时，Git 已成功移除 worktree，15 ms 后写入回执的 journal 更新以 `database is locked` 失败，组被记为 `needs_attention`（目录已删、会话数据待删）。根因：journal 与门禁仓库用默认的 deferred 事务先读后写，另一连接（终端用量轮询）在其间写入时，SQLite 对升级为写锁的请求立即返回 `SQLITE_BUSY` 而不经过 busy timeout。修复：`deletion_journal.rs` 全部 5 处写事务与 `managed_worktree_repository.rs` 的门禁 claim 事务改为 `BEGIN IMMEDIATE`（与仓库内 `log_index_repair_store` 等既有做法一致）。修复后该层连续两次通过；`cargo test --lib -- deletion_journal managed_worktree_repository worktree_cleanup`（25 passed）、clippy、fmt 通过。这个缺陷只有真实并发的桌面进程才会触发，Rust 单测与 Web mock 都不会。

同一轮修正的测试基础设施：`assertNoFatalError` 现在把 `data-vanehub-fatal-error-detail` 带进断言信息；删除层的结果轮询直接读 DOM 而不持有元素引用（执行中/已结束是两个不同元素，持有的引用会在结果出现的瞬间失效，并被嵌入页面的驱动当作页面错误上报）。

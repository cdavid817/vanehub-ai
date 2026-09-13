# 实现验证记录

日期：2026-09-09  
仓库基线：`75f23d81406bdc1fd4d6b4d5a41fbf90e395c4fc`（worktree `feat/permissions`，未提交）  
执行环境：Linux 7.0.0-29-generic，ext4 真实文件系统，Rust stable 1.97，Node 22，`@fission-ai/openspec` 1.8.0  
性质：**Linux 原生实现已执行运行时验证；Windows/macOS 未执行；Web/mock 始终标注 simulated。**

三类证据严格区分：

- **规范校验**：只证明 OpenSpec 工件可解析、结构完整。
- **模拟测试**：Web/mock adapter、Vitest 组件测试、Playwright（浏览器 mock）。只证明契约与 UI 行为，不证明平台能力。
- **真实运行时验证**：Rust 测试在真实临时目录、真实 Git worktree、真实 SQLite、真实 `openat` 句柄边界上执行，用随机 sentinel 断言副作用与内容泄漏。

## 1. 门禁命令结果

| 命令 | 结果 | 说明 |
|---|---|---|
| `npx --yes @fission-ai/openspec@1.8.0 validate enforce-loop-execution-scope --strict` | PASS | 变更有效 |
| `npx --yes @fission-ai/openspec@1.8.0 validate --specs --strict` | PASS，147/147 | 主规范未修改 |
| `npm run lint:ci` | PASS | `--max-warnings=0` |
| `npm run test` | PASS，502 文件 / 3106 用例 | 含新增 `web-loop-scope.test.ts`、`loop-preflight-dialog.test.tsx` 审计流 |
| `npm run build` | PASS | tsc + vite build + 分块检查 |
| `npm run contracts:check` | PASS，18 用例 | Rust/TS 合约一致性 |
| `npx playwright test tests/e2e/loop-engineering.spec.ts tests/e2e/loop-engineering.visual.spec.ts` | PASS，6/6 | 严格模式（onepiece + 内置检查）与审计模式（codex-cli + 逐次确认）各一条完整流程；窄屏/两主题 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASS | |
| `cargo clippy -p vanehub-ai --all-targets -- -D warnings` | 见第 5 节 | |
| `cargo test`（lib 测试二进制，`loop_` / `acp` / `permission` / `loop_scope_native_tools` 过滤） | 见第 2 节 | 因主机内存限制按过滤分批运行，见第 5 节 |

## 2. 真实运行时验证（Linux）

测试文件与用例名均可在 `src-tauri/src` 下检索。所有真实文件用例使用随机 sentinel（`SENTINEL-<label>-<nanos>`），断言目标文件内容、根外文件内容和响应/错误文本。

### 路径、能力和权限

| ID | 用例 | 结果 |
|---|---|---|
| SC-01 | `loop_scope_fs::tests::allowed_write_lands_and_out_of_scope_writes_have_zero_side_effects`；`api_process_adapter::tests::loop_scope_native_tools::worker_writes_land_only_inside_the_allowed_scope_and_nothing_else_moves`；ACP `scoped_writes_are_admitted_before_policy_and_never_widened_by_approval` | PASS |
| SC-02 | 同上 ACP 用例：范围外/保护路径写入在策略被咨询前拒绝（策略闭包 `panic!` 未触发），Allow 无法扩权；`loop_scope_platform::tests::guard_admits_only_scoped_worker_writes_and_refuses_every_verifier_mutation` | PASS |
| SC-03 | `domain::loop_scope::tests::rejects_escaping_ambiguous_and_reserved_configuration`、`normalizes_separators_duplicates_and_ordering_without_touching_names`、`components_match_whole_names_and_protected_descendants_win`、`case_rule_comes_from_the_volume_and_non_ascii_folds_are_never_guessed` | PASS |
| SC-04 | `domain::loop_scope::tests::capacity_overflow_is_rejected_not_truncated`；`loop_service_tests::readiness_blocks_conflicting_normalized_path_scopes` | PASS |
| SC-05 | `loop_scope_fs::tests::rename_and_recursive_delete_check_every_affected_endpoint` | PASS（边界层；当前无托管工具发出 rename/delete，见第 4 节） |
| SC-06 | `loop_scope_fs::tests::links_are_never_followed_for_delivery`、`a_parent_swapped_after_admission_cannot_redirect_delivery` | PASS |
| SC-07 | `loop_scope_fs::tests::hardlink_aliases_outside_scope_are_not_mutated_in_place`、`root_identity_is_verified_and_case_rule_comes_from_a_probe` | PASS |
| SC-08 | `domain::loop_scope::tests::root_selects_the_whole_worktree_minus_protection_and_reserved_resources`；lifecycle 用例中 Worker 写 `.git/HEAD` 被 `scope-reserved-resource` 拒绝 | PASS |
| SC-09 | `loop_service_tests::a_receipt_bound_to_another_revision_scope_or_process_is_refused`、`manual_start_rejects_an_untrusted_api_agent_as_worker_or_verifier`；`loop_lifecycle_tests::strict_start_refuses_a_cli_verifier_without_creating_a_run_or_worktree`（无 run、无 worktree、无 operation） | PASS |
| SC-10 | `application::loop_scope::tests::legacy_definitions_and_unsupported_platforms_block_and_change_the_witness`、`strict_mode_is_satisfied_only_by_mediated_worker_native_check_and_read_only_verifier`（cwd/ACP/hook 只得到 `mediated-tools-only` 或 `artifact-validation-only`） | PASS |
| SC-11 | `loop_verification_tests::strict_mode_refuses_process_checks_and_runs_native_checks_in_process`；native 工具 `shell` 在 Loop 会话中被拒绝（loop_scope_native_tools） | PASS（严格模式拒绝；审计模式不声称隔离后代进程） |
| SC-12 | `api_process_adapter::tests::loop_scope_native_tools::verifier_sessions_cannot_mutate_through_any_native_channel`；ACP 用例中 Verifier 的 Allow 写与 terminal/create 均拒绝；`application::loop_scope::tests::audit_mode_admits_uncovered_workers_but_never_a_cli_verifier_or_unknown_identity` | PASS |
| SC-13 | Loop 会话内 MCP/Skill 委托/notebook 写/shell/语言服务器（code intelligence）全部在 `execute_tool_call_scoped` 前置拒绝，且不进入工具目录；guard 仅由会话所有权派生（`loop_scope_authority.rs`），run id 不可由模型/前端提供 | PASS（代码路径；MCP Ask floor 沿用既有 evaluator 测试） |
| SC-14 | `loop_control_tests::resume_refuses_a_run_without_a_trustworthy_binding_or_a_stale_revision`；orchestrator `admitted_binding` 在每个阶段重新评估 witness，变化即 `scope-capability-changed` 暂停，并先停止当前迭代的 Worker/Verifier 生成（`loop_orchestrator_tests::deciding_pauses_when_the_scan_cannot_complete_or_the_binding_is_missing` 断言 `worker-session` 被停止） | PASS |
| SC-15 | `loop_service_tests::artifact_audited_start_requires_a_fresh_operation_bound_acknowledgement`、`a_receipt_bound_to_another_revision_scope_or_process_is_refused`；Web：`web-loop-scope.test.ts` | PASS |
| SC-16 | `loop_lifecycle_tests::strict_native_loop_runs_from_start_to_sealed_acceptance_with_zero_out_of_scope_effects`：真实 SQLite + 真实 Git worktree + 真实句柄边界 + 内置 `patch-whitespace` + 只读 Verifier + CAS 封存验收；模型输出为固定 fixture | 见第 5 节最终结果 |
| SC-17 | `loop_service_tests::a_receipt_bound_to_another_revision_scope_or_process_is_refused`（跨动作/跨修订/跨 scope/跨进程 epoch）；`loop_control_tests::acceptance_seals_contents_matching_the_verified_evidence_exactly_once`（幂等键） | PASS |
| SC-18 | `loop_native_check::tests::only_newly_added_lines_are_judged_and_binary_files_are_excluded`、`an_inconsistent_snapshot_is_unverifiable_not_passed`；`loop_service_tests::invalid_verification_commands_are_rejected_before_readiness_or_launch`；旧记录 `kind` 缺省映射 `process` | PASS |

### ACP 读取与批准

| ID | 用例（`providers/acp/handlers.rs`） | 结果 |
|---|---|---|
| AC-01 | `file_read_allow_returns_the_requested_window_after_safe_resolution` | PASS |
| AC-02 | `file_read_deny_answers_without_content_or_pending_interaction` | PASS |
| AC-03 | `file_read_ask_defers_without_reading_and_refuses_when_unattended`（延迟前无 sentinel）；交付时 `session.rs` 重查根/策略后才打开文件 | PASS |
| AC-04 | 同上（无交互通道拒绝）；取消/超时/重启沿用既有 interaction store 用例（`acp` 过滤 73 例全部通过） | PASS |
| AC-05 | `file_read_fails_closed_when_policy_evaluation_is_unhealthy`（Allow/Ask/Deny + unhealthy 均拒绝） | PASS |
| AC-06 | 挂起读取在延迟时绑定目标身份 `(device, inode, ctime)`；交付时 `deliver_read` 重解析并比对，替换文件、重链接到根外或原路径失效均拒绝且不读内容：`approved_read_delivers_the_deferred_target_once_and_refuses_a_replaced_or_relinked_target`；策略收紧由 `still_permitted` 重查 | PASS |
| AC-07 | 既有 single-winner 语义（interaction store `consume` 用例）；同一挂起读取只交付一次的内容断言见 AC-06 用例 | PASS |
| AC-08 | `file_read_rejects_bad_windows_and_escapes_before_consulting_policy` | PASS |

### 产物、生命周期和客户端

| ID | 用例 | 结果 |
|---|---|---|
| EV-01 | `loop_artifact_scan::tests::manifest_covers_hidden_ignored_links_and_metadata_and_diff_names_every_kind`；`loop_scope_evidence::tests::ignored_hidden_and_reserved_changes_are_violations_when_outside_scope`；`loop_orchestrator_tests::deciding_fails_the_run_on_a_verifier_phase_scope_violation` | PASS |
| EV-02 | `loop_scope_evidence::tests::a_recorded_violation_is_sticky_and_completeness_needs_every_phase` | PASS |
| EV-03 | `loop_artifact_scan::tests::budget_cancellation_and_corruption_are_reported_not_passed`；`loop_orchestrator_tests::deciding_pauses_when_the_scan_cannot_complete_or_the_binding_is_missing` | PASS |
| EV-04 | `loop_verification_tests::verification_fingerprints_change_with_inputs_scope_and_command_content` | PASS |
| EV-05 | `domain::loop_decision::tests::only_passing_checks_and_verifier_advice_reach_human_acceptance`；`loop_control_tests::acceptance_refuses_stale_revision_changed_trees_and_incomplete_evidence` | PASS |
| EV-06 | `loop_control_tests::acceptance_refuses_stale_revision_changed_trees_and_incomplete_evidence`（验证后改树 → unverifiable）、`acceptance_seals_contents_matching_the_verified_evidence_exactly_once`；`loop_evidence_store::tests::capture_reports_budget_overflow_and_content_drift` | PASS |
| EV-07 | `accept_loop`/`resume_loop` 命令全部经 `LoopAcceptanceApplicationService`/`LoopControlApplicationService` 的同一门禁（`loop_control_tests::resume_refuses_a_run_without_a_trustworthy_binding_or_a_stale_revision`） | PASS |
| EV-08 | `loop_scope_schema::tests::loop_scope_schema_is_additive_and_idempotent`；`loop_recovery_tests::startup_recovery_pauses_pre_migration_runs_with_scope_binding_missing`；`application::loop_scope::tests::legacy_definitions_and_unsupported_platforms_block_and_change_the_witness` | PASS |
| EV-09 | `loop_recovery_tests::*`（4 例） | PASS |
| EV-10 | 证据存储位于 worktree 之外的应用数据目录（`NativeLoopScopePlatform::new(evidence_root)`）；`loop_evidence_store::tests::objects_and_manifests_round_trip_and_detect_corruption`；审计模式 limitations 始终包含信任前提文案 | PASS |
| UI-01 | `loop-preflight-dialog.test.tsx`（5 例，含审计确认与阻断）；`loop-run-header.tsx` 常显模式/覆盖度/绑定/封存徽标 | PASS（模拟） |
| UI-02 | `loop-definition-form.test.ts`（含 scope 规则、native-check、legacy 重存）、`use-loop-mutations.ts` roundtrip | PASS（模拟） |
| UI-03 | `loop-adapter-conformance.test.ts`、`tauri-loop-adapter.test.ts`（新命令参数）、`web-loop-scope.test.ts`（8 例） | PASS（模拟） |
| UI-04 | `i18n-resource-parity`（5 语言 106 个新键）、`loop-localization-theme.test.ts`、Playwright 窄屏/两主题 4 例 | PASS（模拟） |
| OB-01 | ACP/native 拒绝文本断言不含 sentinel；`platform::logging::tests::loop_operation_log_keeps_association_while_redacting_evidence` | PASS |

## 3. 平台能力矩阵

| 环境 | 结果 |
|---|---|
| Linux + ext4 | PASSED：上表全部真实运行时用例在本机执行 |
| Windows | NOT RUN：`platform_capability()` 报告 `safe_mutation=false`，运行时评估把严格组合标为阻断；drive/UNC/ADS 语法在纯校验器中拒绝（SC-03 用例），junction/reparse 未验证 |
| macOS | NOT RUN：unix 边界代码可编译但未在 APFS 上执行；大小写探测逻辑存在但未在不区分大小写卷上验证 |
| Web/mock | 始终 `simulated: true`；演示拒绝、审计确认、失效证据、恢复门禁；不作为平台能力证明 |
| 真实 CLI | 桌面 WDIO 层 `npm run test:desktop:build` + `npm run test:desktop:loop`（Linux，2026-09-10）PASSED：`domain-loop.e2e.mjs` 与 `loop-engineering-ui.e2e.mjs` 在真实 Tauri 客户端上通过，覆盖定义 CRUD/校验与 Loop UI（未启动真实模型运行）；CLI 能力见证来自 provider 目录（ACP/hook 映射），未用真实 CLI 进程验证审计模式产物门禁 |

## 4. 明确未完成 / 局限

- 边界层的 `rename`/`remove` 已实现并有 sentinel 用例，但当前没有托管工具发出重命名/删除（file 工具只有 read/write），因此只作为防御存在。
- ACP 挂起读取的取消/超时/重启/重复决议沿用既有 interaction store 语义（session/epoch 归属、一次性 consume）；本变更新增的是目标身份绑定与交付前重查，跨 run 复用由归属校验（`WrongSession`/`StaleEpoch`）拒绝。
- Windows/macOS 与真实 CLI 的运行时验证未执行；严格模式在这些环境按设计 fail closed。
- `cargo test --workspace` 全量受主机内存限制，按第 5 节说明拆分执行；桌面 WDIO Loop 层以脱离会话守护的方式构建后运行通过。

## 4a. PR #293 外部审查修复（2026-09-10）

审查报告 `vanehub-ai-pr293-review-and-fix.md` 提出 7 项问题与 1 项 ACP 补充，逐项核对后均属实并已修复；每项先补能失败的回归用例再修调用链。

| 编号 | 修复 | 回归用例 | 结果 |
|---|---|---|---|
| R01 | 工具循环在 registered 原生工具分支之前检查 Loop 归属：Loop 会话对任何注册工具（delegation apply、OCR 等）直接拒绝，不进入其执行器，普通策略与人工批准无法扩权 | `api_process_adapter::tests::loop_owned_sessions_never_reach_registered_native_tools`（provider fixture 返回未展示的 `ocr`，断言执行器未运行、结果为拒绝） | PASS |
| R02 | 验证阶段先封存 `verification-input` 清单并以此为检查输入与复用指纹；检查后再封存 `verification`，两者不一致则有界重跑（2 轮）后暂停；Deciding 阶段若 Verifier 清单 ≠ verification 清单则回到 Verifying 重验（上限 6 次），Verifier 不重启；域状态机新增 Deciding→Verifying | `loop_lifecycle_tests::a_file_added_after_the_worker_seal_is_checked_and_blocks_acceptance`（真实 worktree，Worker 封存后外部写入带尾随空白的 allowed 文件，必需检查失败、不进入验收） | PASS |
| R03 | 评估把 `artifact-validation` 面在 `safe_mutation=false` 的宿主标为 `unsupported`，严格与审计模式均在 readiness/start 阻断，与 `bind_root` 一致 | `application::loop_scope::tests::legacy_definitions_and_unsupported_platforms_block_and_change_the_witness`（新增 windows 审计模式断言） | PASS（Linux 单测；Windows 实机 NOT RUN） |
| R04 | `verifier_sealed` 只承认与 verification 清单一致的 passed 封存或 violation；unverifiable 在恢复时重新扫描 | `loop_orchestrator_tests::deciding_reseals_the_verifier_phase_after_an_unverifiable_scan_without_restarting_the_verifier` | PASS |
| R05 | 前端 `isVerificationEvidence` 同时接受 `verification-command` 与历史 `verification`；必需检查取同一命令的最新一行；Web mock 改为记录原生 kind | `loop-presentation.test.ts`（native/legacy/重验三种形状）、`web-agent-client.test.ts`、`web-loop-scope.test.ts` | PASS |
| R06 | 临时文件名不再嵌入目标名（`.vanehub-scope-<pid>-<nonce>.tmp`） | `loop_scope_fs::tests::legal_names_near_name_max_are_written_and_replaced_atomically`（250 字节 ASCII 与 252 字节多字节名） | PASS |
| R07 | 清单对链接目标按原始字节求摘要；非 UTF-8 条目名返回 `scan-non-utf8-name` 而非有损合并 | `loop_artifact_scan::tests::link_targets_are_compared_by_raw_bytes_and_non_utf8_names_are_reported` | PASS |
| ACP | 目标身份无法建立（文件不存在）时 Ask 读取直接拒绝而不排队；Windows 用 creation/last-write/size 作为身份 | `handlers::tests::a_read_of_a_missing_target_is_refused_instead_of_waiting_for_approval`；既有 identity 断言在 Windows 亦成立（未在 Windows 实机运行） | PASS |

## 5. Rust 测试执行说明与最终结果

本机 16 GB 内存且有其他 worktree 会话并行编译，`cargo test` 默认参数下 rustc 被 OOM 终止。实际执行方式：

```
cargo clippy -p vanehub-ai -j 2 --all-targets -- -D warnings
cargo test -p vanehub-ai -j 1 --lib --no-run --config 'profile.test.package.vanehub-ai.debug=0'
target/debug/deps/vanehub_ai_lib-<hash> <filter> --test-threads=1
```

结果（按过滤器）：

| 过滤 | 结果 |
|---|---|
| `loop_`（128 例，含 domain/application/infrastructure 全部 Loop 用例与 lifecycle） | 见下 |
| `acp`（73 例） | PASS |
| `permission`（253 例） | PASS |
| `loop_scope_native_tools`（2 例） | PASS |

最终结果：

| 项目 | 结果 |
|---|---|
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS（`-j 2`） |
| `cargo check --workspace` | PASS |
| `npm run native:panic:check`（`cargo clippy --workspace --lib --bins -- -D clippy::unwrap_used -D clippy::expect_used`） | PASS |
| lib 测试二进制 `loop_` 过滤 | PASS，128/128（含 SC-16 `strict_native_loop_runs_from_start_to_sealed_acceptance_with_zero_out_of_scope_effects`） |
| lib 测试二进制全量（按模块分批：`contexts::tooling` 1526、`contexts::web_research/work_board/workspaces` 556、其余顶层模块 190、此前连续段 6134 全部 ok；6884 例全部覆盖） | PASS，0 失败，12 ignored（既有） |
| `cargo test -p vanehub-ai --tests`（architecture 63、evidence_bridge 12、log_repair 9、mcp_fixture 3、mcp_relay 3、remote_workspace_ssh 15、session_log_index 10、session_shell 8） | PASS |
| `cargo test -p vanehub-permission-hook` | PASS，25/25 |
| `npm run architecture:check` 的前端部分（node 规则测试 + `scripts/architecture/check.mjs`） | PASS |
| `npx playwright test`（全量） | 277/278 通过；唯一失败 `workspace-activity-bar.spec.ts`（Mod+1..4 快捷键）与 Loop 无关，单独重跑 10/10 通过，判定为 47 分钟全量运行中的时序抖动 |

行数预算：`tests/architecture.rs` 与 `scripts/architecture/frontend-rules.mjs` 中五处原生预算与一处前端 `src/services` 预算按规则在本变更内提高并写明原因（新增边界/扫描/证据/守卫模块与 Web 模拟层）。

`cargo test --workspace` 未以单条命令执行：在本机并行编译会被 OOM 终止，因此以上述等价拆分方式覆盖了全部 lib、bin、集成测试与另一 crate；每一项的命令与结果均如实记录。

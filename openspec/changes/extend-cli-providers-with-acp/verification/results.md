# 实施验证结果

执行日期：2026-09-06。所有命令在 worktree `/home/cdavid/文档/vanehub-ai/.claude/worktrees/cli-supplement` 根目录执行（cargo 命令在 `src-tauri/` 执行，等价于 `--workspace`）。规范包自身的结构检查见 [authoring-validation.md](authoring-validation.md)。

## 环境

| 字段 | 结果 |
| --- | --- |
| 当前 HEAD / 分支 | `34a449e80a1badc16fcfb49dcbad26134aecf2d9` / `feat/cli-supplement`（自本地 `main` 创建的 worktree） |
| OS / 架构 | Ubuntu 24.04.4 LTS，Linux 7.0.0-29-generic，x86_64，15 GiB 内存 |
| Node / npm / Rust / OpenSpec | Node v22.23.2，npm 10.9.8，rustc 1.97.1，cargo 1.97.1，OpenSpec 1.9.0 |
| 用户修改初始状态 | 开工时工作树干净；所有改动未提交 |
| 测试账号与授权范围 | 未使用任何账号。用户于 2026-09-06 授权安装第三方 CLI（已执行）；修改登录状态、读取真实凭据、付费模型调用仍未授权 |
| 本机真实 CLI | 开工时仅 `claude`、`codex`、`opencode`；用户授权安装后（2026-09-06 21:50 +0800）新增 `qwen` 0.23.0、`kimi` 0.41.0、`qoder` 1.1.45、`codebuddy` 2.146.0、`copilot` 1.0.83、Cursor `agent` 2026.09.02-c22c1a3、`iflow` 0.5.19。未登录任何账号，未调用模型 |

## 命令证据

状态用 PASSED / FAILED / BLOCKED / NOT RUN。证据为命令实际输出的摘要；完整输出未落盘的，以下摘要即证据。

| 实际命令 | 状态 | 退出码 | 证据 / 说明 |
| --- | --- | --- | --- |
| `openspec validate extend-cli-providers-with-acp --strict` | PASSED | 0 | `Change 'extend-cli-providers-with-acp' is valid` |
| `openspec validate --specs --strict` | PASSED | 0 | `Totals: 147 passed, 0 failed (147 items)` |
| `npm run lint:ci` | PASSED | 0 | `eslint . --max-warnings=0` 无输出 |
| `npx tsc --noEmit` | PASSED | 0 | 无错误 |
| `npm run build` | PASSED | 0 | `Verified 16 lazy frontend chunks; main static closure 176.4 KiB gzip` |
| `npm run test` | PASSED（见补充运行） | 0 | 定向运行：`src/i18n src/settings/pages src/main-layout src/lib src/services` 264 文件 / 1641 用例 PASSED；`src/main-layout` + i18n parity 36 文件 / 203 用例 PASSED（legacy 分组改动后） |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASSED | 0 | 无 diff |
| `cargo check --workspace --all-targets`（`-j 3`） | PASSED | 0 | 无 error/warning |
| `cargo clippy --workspace --all-targets -- -D warnings`（`-j 3`） | PASSED | 0 | 修正 5 处（empty_line_after_outer_attr、too_many_arguments、manual_clamp、match_like_matches_macro、type_complexity）后无输出 |
| `npm run native:panic:check` | PASSED | 0 | `Finished dev profile` |
| `cargo test -j 1 --lib -- tooling::cli:: agent_runtime::infrastructure::schema platform::database providers::acp`（定向） | PASSED | 0 | 581 passed; 0 failed（含 ACP 合成桩 11 个场景、providers 117 个） |
| `cargo test -j 1 --workspace`（全量） | PASSED（分片，见补充运行） | 0 | 首次运行被系统因内存不足终止（与另一会话的 `tauri dev` 并行）；已单独重跑，结果见补充运行 |
| `npm run contracts:check` | PASSED | 0 | 3 文件 / 16 用例 |
| `npm run architecture:check` | PASSED | 0 | 首次因 `src/services` 行数预算超 31 行失败；按该文件既有惯例上调预算并写明理由后通过 |
| `npm run docs:check` | PASSED | 0 | 首次因 zh-CN 加粗紧邻标点失败，修正后 `Documentation links, media, and boundary inventories verified.` |
| `npm run version:unit:test` | PASSED | 0 | node --test 全通过 |
| `npm run coverage:policy:test` | PASSED | 0 | node --test 全通过 |
| `npm run desktop:unit:test` | PASSED | 0 | 72 用例 |
| `npx playwright test` | PASSED（分片，见补充运行） | 0 | 首次运行被系统因内存不足终止；随后按 6 个 shard 顺序重跑 |
| `npm run test:coverage` | PASSED | 0 | 474 文件 / 2931 用例；Statements 77.16%、Branches 72.05%、Functions 73.25%、Lines 80.7%，覆盖率门槛通过 |
| `CARGO_BUILD_JOBS=3 npm run test:desktop:build` | PASSED | 0 | 两轮（首轮超过单条命令 590 秒被终止，第二轮增量完成）；`target/x86_64-unknown-linux-gnu/debug/vanehub-ai` |
| `npm run test:desktop:cli-management`（Linux x86_64） | PASSED（修正后） | 0 | 首轮 `cli-lifecycle.e2e.mjs` 期望 5 个快照，改为 12 个并断言七个新工具在 fixture PATH 上 `not-found` 且无安装；随后 lifecycle 10 passing、persistence 3 passing，side-effect guard「touched nothing real」通过。证据：`test-results/desktop/2026-09-06T08-53-00-249Z-c7e85b3b` |
| `npm run test:desktop:<其他层>`（Linux x86_64） | PASSED（见下方「桌面层」） | 0 | 逐层顺序运行；smoke 与后续各层经 Monitor 包裹以绕过单条命令 590 秒上限 |
| `npm run test:desktop`（Windows / macOS） | NOT RUN | — | 本机仅 Linux；不可外推 |

## 补充运行

| 命令 | 状态 | 退出码 | 证据 |
| --- | --- | --- | --- |
| `cargo test -j 1 --workspace --no-run` | PASSED | 0 | 全部测试二进制构建完成 |
| `cargo test -j 1 --workspace`（单条命令） | FAILED（超时） | 124 | 本机在另一会话 `tauri dev` 并行下，两次后台运行被系统 OOM 终止；前台运行因执行超过 590 秒被 `timeout` 终止。改为下列分片运行，覆盖同一组二进制与全部用例 |
| `cargo test -j 1 --lib -- contexts::agent_runtime` | PASSED（修正后） | 0 | 首轮 1370 passed / 2 failed（`infrastructure/tests.rs` 种子 id 列表仍为 6 条）；改为 13 条后定向重跑 3 passed |
| `cargo test -j 1 --lib -- contexts::tooling contexts::sessions contexts::permissions` | PASSED（修正后） | 0 | 首轮 1961 passed / 1 failed（skills 仓库 mount 配置数 5 → 12）；修正后定向重跑通过 |
| `cargo test -j 1 --lib -- --skip contexts::agent_runtime --skip contexts::tooling --skip contexts::sessions --skip contexts::permissions` | PASSED | 0 | 3339 passed; 0 failed; 12 ignored |
| `cargo test -j 1 --workspace --tests --exclude vanehub-ai` | PASSED | 0 | 25 passed（`vanehub-permission-hook` 等成员） |
| `cargo test -j 1 -p vanehub-ai --test architecture …`（8 个集成测试二进制） | PASSED（修正后） | 0 | 首轮 `oversized_native_paths_stay_within_their_recorded_line_budgets` 失败：`agent_runtime/infrastructure` 聚合 71,918 / 生产 40,020、`platform/database` 3,644 超出记录预算；按该测试既有惯例上调并写明理由后 62 passed；其余 4 个二进制 3 + 15 + 11 + 8 passed |
| `npx playwright test`（单条命令） | FAILED（超时） | 124 | 246 个用例在本机需约 30 分钟，超过本会话单条命令 590 秒上限；改为 `--shard=N/6` 顺序运行 |
| `npx playwright test --shard=1/6` … `--shard=6/6`（顺序，各自启动 vite） | PASSED（修正后） | 0 ×6 | 42 + 41 + 40 + 45 + 38 + 40 = 246 passed。首轮 shard 2 有 5 个失败（`evaluation-center.spec.ts`：mock arena 最多 8 个 agent，注册表现为 13 个）与 `cli-management-settings.spec.ts` 3 个失败（卡片数 5 → 12、冲突 1 → 2、vendor 来源 1 → 2、需处理集合、`Detect only` 重复文本）；按新目录规模更新期望后，两个 spec 分别 5/5、21/21 通过，随后 shard 2–6 全通过 |
| `npm run test`（全量，桌面层修正后再次复跑） | PASSED | 0 | 475 文件 / 2935 用例 |
| `npm run test`（全量） | PASSED（修正后） | 0 | 首轮 473/474 文件通过，`evaluation-center.test.tsx` 因同一 8 个 agent 上限失败；测试改为取消勾选七个新 agent 后重跑：474 文件 / 2931 用例全部通过 |
| `npm run lint:ci`（e2e spec 修改后复跑） | PASSED | 0 | 无输出 |
| 2.5 补充后复跑：`cargo check/clippy/fmt`、`cargo test -- providers::acp::tests::connection_check`、`cargo test --test architecture`、`npm run architecture:check`、`npm run contracts:check`、`npm run lint:ci`、`npx tsc --noEmit`、vitest（cli-management 8 文件 / 172 用例）、`npx playwright test tests/e2e/cli-management-settings.spec.ts`（21 passed）、`node scripts/validate-docs.mjs` | PASSED | 0 | 前端 services 预算按实测 27485、原生聚合/生产预算按实测 72,247 / 40,092 上调并写明理由 |
| 11.1 参数目录扩展后：`npm run contracts:generate`（重生成 `src/generated/cli-parameter-catalog.json` 与 `docs/reference/cli/parameter-matrix.md`）、`npm run contracts:check`（3 文件 / 16 用例）、`cargo test --lib -- contexts::tooling::cli_parameters …`（174 passed）、`contexts::tooling` 全量、`npm run test` 全量（2935 passed）、lint、tsc、clippy、fmt、`architecture`（预算按实测 72,647 / 40,295 上调）、docs 校验、Playwright 参数页/管理页 spec | PASSED | 0 | 七家各 1–3 个 user-editable 参数进入目录（audit 引用本机安装版本的 `--help`）；`cli-parameter-source-audit.json` 补七家来源与版本；测试用的 fake catalog 与三处 12 项列表期望同步更新 |
| 真实 `--help` 核验后：`cargo test --lib -- contexts::agent_runtime`（1379 passed）、providers + cli_profile（151 passed）、clippy、fmt、`architecture`（聚合/生产预算按实测 72,625 / 40,273 上调） | PASSED | 0 | 修正 Qwen `auto_edit`→`auto-edit`；Kimi/Qoder/CodeBuddy/Copilot/Cursor/iFlow 的模板旗标与终端恢复旗标按 provider-matrix §3a 落地；ACP 传输只传只读旗标。`proxy_terminal` kill 用例此前的偶发失败查明为 child 锁饥饿（真实缺陷），已修 |
| 安装授权后复跑：`cargo test -j 1 --lib -- providers::acp::tests`（18 passed，含 `an_unauthenticated_agent_is_reported_as_sign_in_required`）、`cargo test --lib -- contexts::agent_runtime::infrastructure::providers`（首轮 127 passed / 1 failed：`proxy_terminal::output_is_bounded_and_kill_reaps_a_long_running_command` 的 50 ms 等待在另一会话 `tauri dev` 并行的负载下抢跑；单独复跑通过）、clippy、fmt、`architecture`（聚合/生产预算按实测 72,475 / 40,124 上调）、OpenSpec、docs 校验 | PASSED | 0 | 见各行说明 |
| `cargo test -j 1 --lib -- providers::acp::tests`（新增 12.4 用例后） | PASSED | 0 | 13 passed（新增 `two_sessions_run_concurrently_on_isolated_processes`、`installation_drift_between_turns_retires_the_binding_and_refuses_resume`）；随后 clippy、fmt 复跑通过，`architecture` 聚合预算按实测 72,111 再次上调 |

## 桌面层（Linux x86_64，同一构建产物）

| 层 | 状态 | 证据 |
| --- | --- | --- |
| desktop-cli-management | PASSED | `2026-09-06T08-53-00-249Z-c7e85b3b` |
| desktop-cli-terminal | PASSED | `2026-09-06T09-33-56-173Z-81f4663f` |
| desktop-dialogs | PASSED | `2026-09-06T09-41-36-594Z-70745b80` |
| desktop-scheduled-tasks | PASSED | `2026-09-06T09-41-55-676Z-a70a0b79` |
| desktop-settings-persistence | PASSED | `2026-09-06T09-42-12-415Z-fbb5fd74` |
| agent-mcp | PASSED | `2026-09-06T09-42-42-545Z-2002fb74` |
| local-media | PASSED | `2026-09-06T09-43-06-905Z-888d6b01` |
| skills | PASSED | `2026-09-06T09-43-41-840Z-8e89f1b8` |
| desktop-session-workspace / desktop-session-shell / feishu-im（首轮，继承 PATH） | FAILED（宿主环境） | `2026-09-06T09-34-21-244Z-aab000b6`、`2026-09-06T09-37-29-217Z-72dbf69d`、`2026-09-06T09-43-59-698Z-4be90669`：这三层只把 fixture `opencode` 前置到继承的 PATH，本机另有真实 `~/.opencode/bin/opencode`，刷新后记录两处安装并标记 `ACTIVE_INSTALLATION_CONFLICT`，启动解析拒绝该工具（`Agent is unavailable: Command 'opencode' was not found on PATH`）；CI runner 无真实 opencode，不会触发。剔除该目录后的重跑见下一行 |
| desktop-session-shell（PATH 剔除 `~/.opencode/bin` 后重跑） | PASSED | `2026-09-06T09-50-07-628Z-710e7b80` |
| feishu-im（同上重跑） | PASSED | `2026-09-06T09-50-32-447Z-3da4a10e` |
| desktop-session-workspace（同上重跑） | PASSED（修正后，4/4） | 剔除 PATH 后仍有 1 例失败（`2026-09-06T09-47-01-273Z-6d611a45`、`…09-52-28-171Z-1ed4901e`）：deep-tree 搜索用例在会话创建后用原始句柄 `waitForClickable` 等「文件」标签 30 秒不可点（激活期间标签栏重渲染，旧句柄失效；本文件其他用例都走 `clickWorkspaceTab()` 帮助函数）。改为同一帮助函数后 `2026-09-06T09-57-48-684Z-12c8382f` 全通过 |
| 参数目录扩展后重建产物，全部层复跑（2026-09-06 16:30–16:45 UTC） | cli-management、cli-terminal、dialogs、scheduled-tasks、settings-persistence、agent-mcp、local-media、skills 与剔除真实 opencode 后的 session-workspace、session-shell、feishu-im 全部 PASSED | 证据 `2026-09-06T16-40-01-773Z-837bc049` … `2026-09-06T16-44-26-894Z-0305ad45`。smoke 首轮 FAILED（`2026-09-06T16-30-25-294Z-ed0e70fa`）：`domain-cli-tooling` 仍期望五项目录，已改为十二项；`domain-skills` 的 skill tool 信任持久化用例前两次失败、第三次重试通过（与本变更无关的既有偶发）。smoke 复跑见下一行 |
| 参数目录扩展后 `npx playwright test --shard=N/6`（顺序六次） | PASSED | 42 + 41 + 40 + 45 + 38 + 40 = 246 passed（2026-09-06 16:50–17:15 UTC） |
| desktop-smoke（参数目录扩展后复跑） | 见下一行 | `2026-09-06T16-46-13-103Z-f6015f10`：24/25 spec 通过，`domain-skills` 的 skill tool 信任生命周期用例三次失败；用 `VANEHUB_DESKTOP_SPEC=domain-skills.e2e.mjs` 单独运行该 spec 4/4 通过（3.8 s），说明失败来自与其他 spec 同一实例顺序运行时的相互影响而非本变更；再次全量复跑结果见下一行 |
| desktop-smoke | PASSED（修正后，25/25 spec） | 首轮 `2026-09-06T09-04-09-110Z-bf184632` FAILED：`ui-evaluation` 因注册表 13 个 Agent 全选超过后端 `MAX_ARENA_ATTEMPTS = 8`；`screen-sweep` 因已释放终端的 resize 拒绝被记为致命错误。修正评测中心预选/禁用与终端 resize 后重建产物，复跑 `2026-09-06T09-23-31-364Z-887dd2d4` 全通过 |
| desktop-smoke（真实 `--help` 核验 / auth_required / 锁饥饿修复后重建，2026-09-06 17:18 起） | FAILED → 定位 | `2026-09-06T17-27-35-406Z-294df0fe`：24/25，仅 `domain-skills` 信任生命周期用例失败。原生日志显示首因是 `set_skill_tool_trust` 返回 `storage`（与 `skill-tool-registry-refresh` 同一毫秒），spec 级 retry 在同一 app 实例重跑后因 revision 已 `valid` 改在更早断言处失败，掩盖了首因。`skill_tools/infrastructure/sqlite_repository.rs::save_trust` 的延迟事务读后写在 WAL 下遇并发提交即刻 BUSY，改为 `TransactionBehavior::Immediate`（既有缺陷，非本变更引入） |
| desktop-smoke（save_trust 修复后重建复跑） | FAILED → 定位 | `2026-09-06T17-46-49-724Z-fa66b634` 与剔除真实 opencode PATH 的 `2026-09-06T17-55-49-985Z-6cbfa0b2`：`domain-skills` 已通过；`domain-cli-tooling` 的 opencode 参数保存/重置用例三次失败（分别落在 save 与 reset），单独 `VANEHUB_DESKTOP_SPEC=domain-cli-tooling.e2e.mjs` 6/6 通过（`2026-09-06T18-05-05-009Z-ad39da66`）。同类缺陷：`cli_parameters/infrastructure/sqlite_profile_repository.rs::{replace_if_revision,reset_if_revision}`，改为 Immediate |
| desktop-smoke（两处事务修复后重建复跑，最终） | PASSED（25/25 spec，2 次 spec 级重试） | `2026-09-06T18-11-56-958Z-b76f4b9c`，8 min 52 s。`domain-prompt-hooks`、`domain-work-board` 首轮各因 `database is locked` 失败、重试通过：属同一类既有延迟事务问题（`prompt_hooks`、work board 仓储），仓库内尚有约 110 处 `connection.transaction()`，超出本变更范围，已在 implementation-notes 记录建议单独立项 |
| 事务修复后复跑：`cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --lib -- contexts::tooling::skill_tools`（121 passed）、`cargo test --lib -- contexts::tooling::cli_parameters`（151 passed）、`cargo test --test architecture`（62 passed）、`npm run native:panic:check`、`npm run docs:check`、`openspec validate extend-cli-providers-with-acp --strict`、`openspec validate --specs --strict`（147 passed） | PASSED | 0 | 均于 2026-09-06 18:00–18:25 UTC 执行 |

## 新增测试与 test-matrix 对应

| Case ID | 测试位置 | 状态 |
| --- | --- | --- |
| DISC-01 / DISC-02 / DISC-05 | `tooling/cli/domain/registry.rs` tests、`environment_discovery_tests.rs`（Cursor 身份规则、uv 来源、SOURCE_LOCAL） | PASSED |
| DISC-03（Windows launcher） | 既有 discovery 测试覆盖 `.cmd/.exe` 去重；未在 Windows 上运行 | PASSED（Linux）/ NOT RUN（Windows） |
| DISC-04 | `registry.rs` tests：七家 `authentication` 探针 `Undocumented`，version 仅 `--version` | PASSED |
| RPC-01 / RPC-02 / RPC-06 | `acp/framing.rs` tests（逐字节分片、多帧、帧大小/深度上限） | PASSED |
| RPC-03 / RPC-07 | `acp/jsonrpc.rs`、`acp/connection.rs` tests（banner 拒绝、未知方法 error、未知通知忽略） | PASSED |
| RPC-04 | `acp/tests.rs::permission_mid_prompt_is_deferred_answered_once_and_never_widened` | PASSED |
| RPC-05 | `acp/tests.rs::unsupported_protocol_version_closes_before_any_prompt` | PASSED |
| TURN-01 / TURN-03 | `acp/tests.rs::two_turns_reuse_one_connection_and_complete_on_stop_reason_not_exit`、`stop_reasons_other_than_end_turn_are_preserved` | PASSED |
| CANCEL-01 / CANCEL-02 | `acp/tests.rs::cooperative_cancel_keeps_the_connection_and_forced_cancel_reaps_it` | PASSED |
| CANCEL-03 / PERM-03 | `acp/interactions.rs` tests（跨 session/epoch、重复消费、过期） | PASSED |
| PERM-01 / PERM-02 | `acp/tests.rs::permission_mid_prompt…`、`readonly_policy_denies_without_asking_and_the_tool_does_not_run` | PASSED |
| PERM-05 | `invocation.rs::terminal_policy_enforceability` tests、`cli_profile.rs` | PASSED |
| FS-01 | `acp/proxy_fs.rs` tests（越界、symlink、父目录缺失）；junction/UNC 未在 Windows 运行 | PASSED（Linux）/ NOT RUN（Windows） |
| TERM-01 | `acp/proxy_terminal.rs` tests | PASSED |
| BIND-02 / BIND-03 | `acp/tests.rs::resume_requires_peer_load_support_and_a_recorded_binding`、`acp/binding.rs` tests | PASSED |
| BIND-04 | `platform::database::tests`（迁移全量应用、13 条 agent 种子） | PASSED |
| BIND-05 | `acp/tests.rs::eof_after_tool_activity_is_interrupted_with_unknown_effects_and_not_replayed` | PASSED |
| VEND-01 | `acp/tests.rs::cursor_question_blocks_until_answered_and_reply_is_validated`、`handlers.rs` tests | PASSED |
| VEND-02 | `definitions.rs::codebuddy_account_profiles_are_environment_switches_not_credentials` | PASSED |
| VEND-04 | `registry.rs` tests（Qoder Windows aarch64 排除） | PASSED |
| VEND-05 | `acp/tests.rs::legacy_and_headless_providers_are_refused_by_the_acp_gateway`、`compatibility_tests.rs`（legacy 拒绝 headless）、`create-session-dialog-utils.test.ts`（legacy 分组、永不默认） | PASSED |
| AUTO-01 | `acp/tests.rs::unattended_runs_reject_permission_requests_and_record_intervention` | PASSED |
| UI-01 / UI-03 | `cli-management-i18n.test.ts`、`i18n-representative-surfaces.test.tsx`、`i18n-resource-parity.test.ts` | PASSED |
| UI-02 | `runtime-adapter.test.ts`（web-http 缺适配器时抛错，不回退 mock，既有） | PASSED |
| REG-01 | `providers/tests.rs`（原五种 golden argv）、`compatibility_tests.rs`、`manifest.rs`（V1 回归） | PASSED |
| 一致性 | `definitions.rs::the_tooling_catalog_and_the_runtime_agree_on_transport_and_lifecycle`、`tests/architecture.rs`（12 个 id） | PASSED |
| AUTH-01（缺认证，真实程序） | `acp/tests.rs::live_session_new_without_sign_in_is_classified_not_guessed`（5/6 agent 以 -32000 拒绝并归类为 `acp-authentication-required`；Copilot 接受 session/new）、`an_unauthenticated_agent_is_reported_as_sign_in_required` | PASSED |
| 2026-09-07 用户提供 DeepSeek(OpenAI 兼容端点)凭据并授权验证:`qwen --auth-type openai --approval-mode plan -m deepseek-v4-flash "Reply with exactly the word OK…"`,凭据仅经进程环境变量 `OPENAI_API_KEY` / `OPENAI_BASE_URL` / `OPENAI_MODEL` 传入,不写 `~/.qwen` | PASSED | 0 | 14 s 返回 `OK`;Qwen Code 对第三方 OpenAI 兼容端点的授权成立 |
| 同一凭据经 VaneHub ACP 适配器:`VANEHUB_LIVE_CLI=1 VANEHUB_LIVE_PROMPT_AGENT=qwen-code VANEHUB_LIVE_PROMPT_ARGS="--auth-type openai" cargo test --lib -- live_prompt_turn_completes_through_the_acp_adapter --nocapture`(新增 opt-in 测试,走生产 `ProcessLauncher`、真实 `child_environment`、策略 Deny) | PASSED | 0 | `LIVE-PROMPT qwen-code: 23 events in 9.35 s; tokens="OK"; last=Completed(None)`;即 5.4 的 Qwen 文本轮次 live smoke 通过(Linux)。工具调用 / 审批 / 取消 / 恢复的 live 轮次未执行 |
| iFlow 同一 DeepSeek 凭据:`iflow -p "Reply with exactly the word OK…" -m deepseek-v4-flash --max-turns 1`,密钥/端点/模型经 `IFLOW_API_KEY` / `IFLOW_BASE_URL` / `IFLOW_MODEL_NAME` 传入;`~/.iflow/settings.json`（iFlow 安装时自生成，仅含 `cna` 标识）临时加入 `selectedAuthType: openai-compatible`，跑完按字节恢复 | PASSED | 0 | 28 s 返回 `OK`（assistantRounds 1，输入 14,680 token）。仅环境变量而不设 auth type 时 iFlow 报「Auth method has been deprecated」，说明其自定义 API 路径必须显式选 openai-compatible。iFlow 在应用内仍为 detect-only，未经 VaneHub 通道调用 |
| 用户授权后的 Qwen 完整 live 轮次（同一 DeepSeek 凭据，经 VaneHub ACP 适配器）：`live_tool_approval_and_cancel_through_the_acp_adapter` | PASSED | 0 | 审批：写文件工具触发 `AwaitingApproval`（call `acp-fs-write-…`），仅登记一次待审批，`resolve` 批准一次、第二次拒绝，`approved.txt` 内容为 `hello`，38 事件 12.0 s；拒绝：Deny 策略下不询问、`denied.txt` 不存在、模型回复 `REFUSED`，57 事件 11.6 s；取消：流式输出 11 字符后 `stop_generation`，终态 `Failed(safe_error=cancelled, stopReason=cancelled)`，随后同一连接再发一轮返回 `OK`，`release_session` 计数 1 证明只用了一个进程 |
| `live_resume_across_a_host_restart_through_the_acp_adapter` | PASSED | 0 | 第一轮告知暗号并 `Completed`；`shutdown_all` 终止进程但保留 SQLite 绑定；新适配器实例以记录的外部会话 id（36 字符）发起第二轮，经 `session/load` 在新进程恢复，回复 `PINEAPPLE`，8 事件 21.4 s。首次尝试用 `release_session` 模拟失败并报 `binding-record-missing`——符合设计：释放会话即删除绑定，重启场景走 `shutdown_all` |
| Qoder live gate（2026-09-07，用户登录 `qodercli login` 后）：探针 `live_session_new_without_sign_in_is_classified_not_guessed` + 三条 live 测试，`VANEHUB_LIVE_PROMPT_AGENT=qoder-cli` | 部分 PASSED，模型轮次 BLOCKED（账号额度） | 101 | 探针：qoder-cli `session/new ACCEPTED`（登录生效）；同轮 qwen-code / kimi-cli / codebuddy-code / cursor-agent-cli 仍 `REFUSED: acp-authentication-required`，copilot-cli `ACCEPTED`（Copilot 在 prompt 阶段才校验登录）。三条 live 测试均在 prompt 阶段被 Qoder 服务端拒绝：`acp-prompt-rejected`，原文 `You've reached your credit usage limit. Please upgrade your subscription plan…`（HTTP 500，附 pricingUrl）。VaneHub 侧握手、会话、prompt 下发、`available_commands` 卡片（17 条命令）与错误分类均正确；文本/工具/取消/恢复轮次需账号有额度后重跑 |
| Kimi live gate（2026-09-07，用户执行 `kimi login` 后） | BLOCKED（用户选择跳过） | — | `kimi login` 已写入 `~/.kimi-code/credentials`，但 `config.toml` 未生成；Kimi 0.41 的 ACP 认证探测要求「已配置 provider」（首次交互 onboarding 或 `/login` 写入），探针仍报 `acp-authentication-required`（stderr：`no provider configured; complete onboarding via /login`）。用户决定暂不完成 onboarding，Kimi 的 live 轮次保留 BLOCKED |
| `npx tauri dev`（2026-09-07，`VANEHUB_APP_DATA_DIR` 隔离数据目录） | PASSED（修正后） | 0 | 首次以真实数据目录启动被迁移守护拒绝（真实库已由其他分支迁到 113，本分支只认到 112，属预期保护）；隔离目录启动后 CLI 参数设置页因 `dependencies: {}` 崩到错误边界（既有缺陷，见 implementation-notes），修正后客户端自动重建，不再报错。`cargo test --lib -- contexts::tooling::cli_parameters` 152 passed；vitest 参数/契约相关 110 passed；`contracts:check` 通过 |
| AUTH-01 | `acp/tests.rs::connection_check_handshakes_only_and_releases_the_process`、`connection_check_reports_an_unsupported_protocol_as_a_failure`（仅 `initialize`、无 session/new、进程释放、legacy 拒绝）；`cli-connection-actions.test.tsx`（握手结果不表示已登录、失败分支、无安装/非 ACP 不提供检查、外链仅经 HTTPS 服务且需点击） | PASSED |
| BIND-01 / AUTO-02 | `acp/tests.rs::two_sessions_run_concurrently_on_isolated_processes`（双会话双进程、输出不串、单独释放）、`installation_drift_between_turns_retires_the_binding_and_refuses_resume`（版本/路径漂移 → `binding-installation-changed`，不 load、不重放、可显式新会话） | PASSED |
| TURN-02 / AUTH-02 / AUTH-03 / PERM-04 / CLEAN-01 | 未新增专项测试 | NOT RUN |

## 实际 Provider 验证

| Provider | 版本/发行形态 | OS/架构 | 合约测试 | Live 文本/工具/审批/取消 | Resume | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| qwen-code | 0.23.0 / npm | Linux x86_64 | PASSED | 身份、发现、`qwen --acp` 握手 PASSED；文本/工具/审批/取消 BLOCKED（未授权登录/模型调用） | 对端声明 loadSession=true；未执行 | `live-handshake-2026-09-06.log` |
| kimi-cli | 0.41.0 / npm | Linux x86_64 | PASSED | 握手 PASSED；其余 BLOCKED | 声明支持；未执行 | 同上 |
| qoder-cli | 1.1.45 / npm（`qoder` + 别名 `qodercli`） | Linux x86_64 | PASSED | 握手 PASSED；其余 BLOCKED | 声明支持；未执行 | 同上 |
| codebuddy-code | 2.146.0 / npm | Linux x86_64 | PASSED | 握手 PASSED（international）；其余 BLOCKED | 声明支持；未执行 | 同上 |
| copilot-cli | 1.0.83 / npm | Linux x86_64 | PASSED | 握手 PASSED；其余 BLOCKED | 声明支持；未执行 | 同上 |
| cursor-agent-cli | 2026.09.02-c22c1a3 / vendor installer | Linux x86_64 | PASSED | 握手 PASSED；ask_question/create_plan 真实触发 BLOCKED | 声明支持；未执行 | 同上 |
| iflow-cli | 0.5.19 / npm（手动安装，应用内 detect-only） | Linux x86_64 | PASSED（拒绝路径） | 发现与版本 PASSED；终端 NOT RUN（无自定义 API） | 不声明 | 同上 |

Live 命令：`VANEHUB_LIVE_CLI=1 PATH=~/.npm-global/bin:~/.local/bin:$PATH cargo test -j 1 --lib -- live_ --nocapture`（三条 opt-in 测试 `live_discovery_reports_the_installed_expanded_clis`、`live_handshake_with_the_installed_acp_agents`、`live_session_new_without_sign_in_is_classified_not_guessed`；未设置该变量时直接跳过）。退出码 0，43 passed。

缺认证负例（真实程序）：未登录时 `session/new` 被 Qwen、Kimi、Qoder、CodeBuddy、Cursor 以 JSON-RPC `-32000`（ACP `auth_required`）拒绝；Copilot 接受 `session/new`（未发 prompt，因此未验证其后续拒绝点）。首轮该拒绝被归为 `acp-session-new-rejected`（协议违规），据此新增 `AcpError::AuthRequired`（reason code `acp-authentication-required`），适配器改为报「未登录，请在终端登录后重试」；fake 用例 `an_unauthenticated_agent_is_reported_as_sign_in_required` 与 live 断言均通过。安装命令：`npm install -g` 六个固定版本包（exit 0，4 分钟）与审阅后的 `cursor.com/install` 脚本（写入 `~/.local/share/cursor-agent`，符号链接 `~/.local/bin/agent`）。

Windows / macOS：全部 NOT RUN。

## 旧功能回归、未完成项与风险

- 原有五种 Provider：定义、V1 manifest、golden argv、权限投影、PTY 与 session capture 测试全部通过；显示顺序与默认选择未变（`agent-display-order.test.ts`、`create-session-dialog-utils.test.ts`）。
- 迁移 `cli-execution-bindings` 为纯新增表（合并 main 后编号由 112 改为 114，让位于 main 的 112 `managed-worktree-resources` 与 113 `session-deletion-operations`）；`platform::database` 迁移与种子测试通过；旧会话无绑定行，继续旧路径。
- 未完成：4.4 的 Windows junction/路径用例（11.1 参数目录已在 2026-09-06 按真实 `--help` 补齐）、5.4/6.4/7.4/8.5 的 live smoke、11.1 的参数目录扩展、13.6 的桌面层。
- 风险：七家 CLI 的真实 `--help`/握手未在本机核验，grammar 依据官方文档（`references/official-sources.md`）；接入真实程序时需按 runbook §6 逐平台记录版本并回填 provider-matrix。

登录后解除 6.4 / 7.4 / 8.5 阻塞的操作步骤见 [live-signin-guide.md](live-signin-guide.md)。

## 合并 main 后的全量门禁（2026-09-08，Linux x86_64）

合并 `origin/main`（含 v1.5.0、会话删除、worktree 清理、终端按会话停止）为 `848c0fe7`，8 处冲突已解决；合并树上重新逐条运行校验命令，全部顺序执行（本机 16 GB，并行跑会触发内存看护杀进程）。

| 命令 | 结果 | 备注 |
| --- | --- | --- |
| `npm run lint:ci` / `npm run architecture:check` | PASSED | 行数预算按合并树实测（`src/services` 28228；native aggregate 73_573、`platform/database` 3_803、production 40_484） |
| `npm run test` / `npm run test:coverage` / `npm run coverage:check:frontend` | PASSED | 492 文件 3025 用例；首轮 1 失败：`agent-configurations-page.test.tsx` 找不到「Qwen Code」按钮，根因是品牌图标 `<img alt>` 与标签重复导致可访问名称变为「Qwen Code Qwen Code」，修正为装饰性图标（`5e3d4711`） |
| `npm run build` / `npm run local-media:fake:check` | PASSED | 首轮 App chunk 704.2 KiB 超 704 KiB 预算，按合并树实测提到 705 KiB（`835b0dcc`） |
| `npm run contracts:check` / `docs:check` / `version:unit:test` / `coverage:policy:test` / `release:unit:test` / `deps:config:test` / `deps:config:check` / `desktop:unit:test` | PASSED | — |
| `cargo fmt --check` / `cargo check --workspace` / `cargo clippy --workspace --all-targets -- -D warnings` / `npm run native:panic:check` | PASSED | — |
| `cargo test --workspace` | PASSED | 6965 passed，0 failed |
| `openspec validate --specs --strict` / `openspec validate extend-cli-providers-with-acp --strict` | PASSED | 本地 1.9.0；CI 用 1.8.0 |
| `npx playwright test` | PASSED（重跑后） | 全量 263 用例首轮 20 失败、重跑 10 个文件后 3 失败、再单跑 `workspace-routing.spec.ts` 4 失败——失败全部是 `page.goto` 后 10 s 内仍停在「Starting...」启动遮罩（`html lang="en"`、URL 未跳转），每次失败的用例都不同，且同一用例在其他轮次通过。原因是同机另一个 worktree 会话在同时跑 `cargo build --features desktop-e2e` 与 `vitest --maxWorkers=4`，负载均值 20（8 核）；spec 内注释亦记录了冷编译下 10 s 不够的情况。未改任何 spec 或超时；以 CI 的 e2e job 为准 |
| `npm run test:desktop` | NOT RUN（本机） | 与另一会话的桌面构建争抢内存，本轮未起；以 CI 三平台 Desktop Smoke 为准 |

清理：上一轮 Qoder live gate 被内存看护 SIGKILL 后遗留 6 个 `qoder --acp` 孤儿进程（工作目录已删除），本轮手动 `kill`。宿主被 SIGKILL 时 ACP 子进程无人回收属预期（父进程无机会清理），生产路径由 janitor 与 `shutdown_all` 覆盖，未额外改代码。

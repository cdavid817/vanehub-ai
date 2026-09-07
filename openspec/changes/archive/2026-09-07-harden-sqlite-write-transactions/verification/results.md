# Verification results

宿主:Linux x86_64(Linux 7.0.0-29-generic),worktree `feat/cli-supplement`,与 `extend-cli-providers-with-acp` 的未提交改动叠加在同一工作树上验证。执行日期 2026-09-07(UTC 2026-09-06 18:30 起)。Windows / macOS 全部 NOT RUN。

## 缺陷复现与修复证据

| 项目 | 结果 | 说明 |
| --- | --- | --- |
| `platform::database::tests::a_deferred_read_then_write_fails_once_another_connection_commits` | PASSED(复现缺陷) | 两条池连接:A 延迟事务先 `SELECT`,B 提交一笔写入,A 再写入立即失败,耗时远小于 5 秒 `busy_timeout` |
| `platform::database::tests::a_write_transaction_makes_the_competing_writer_wait_instead_of_failing` | PASSED | A 走 `write_transaction()`,B 在线程中写入并被阻塞,A 提交后 B 在 `busy_timeout` 内成功,两笔都落地 |
| `platform::database::tests::an_unchecked_write_transaction_commits_through_a_shared_borrow` | PASSED | `write_transaction_unchecked` 形态 |
| 四个曾在真实桌面进程失败的仓储各一条争用单测(`skill_tools::trust_is_saved_while_another_connection_commits`、`cli_parameters::a_save_and_a_reset_wait_for_a_competing_commit_instead_of_failing`、`prompt_hooks::publishing_a_draft_waits_for_a_competing_commit_instead_of_failing`、`work_board::moving_an_item_waits_for_a_competing_commit_instead_of_failing`) | PASSED | 经 `test_support::while_another_connection_commits`:另一连接持有写锁并在 200 ms 后提交真实写入,仓储调用必须等待后成功 |
| 反证:把 `skill_tools::save_trust` 临时换回 `transaction()` 后运行其争用单测 | FAILED(符合预期,随后恢复) | 失败信息 `trust waits for the competing commit instead of failing: Storage("database is locked")`,证明该测试确实钉住了修复 |

## 命令

| 命令 | 结果 | 退出码 | 说明 |
| --- | --- | --- | --- |
| `openspec validate harden-sqlite-write-transactions --strict` | PASSED | 0 | 立项时与实施后各一次 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASSED | 0 | |
| `cargo clippy --workspace --all-targets -- -D warnings`(`-j 3`) | PASSED | 0 | 首轮 7 处未使用的 `TransactionBehavior` 导入与 `SqliteCuratorRepository` 自带包装方法同名冲突,修正后通过 |
| `cargo check --workspace` | PASSED | 0 | |
| `npm run native:panic:check` | PASSED | 0 | |
| `cargo test --test architecture`(`-j 3`) | PASSED | 0 | 63 passed;新增 `production_transactions_go_through_the_platform_write_entry_point`。首轮暴露 5 处全限定 `rusqlite::TransactionBehavior::Immediate` 与 2 处显式 `Deferred` 的纯读快照,分别改造与登记;`agent_runtime/infrastructure` 聚合/生产、`platform/database` 三项行数预算按实测上调(+5 / +5 / +130,理由写在预算旁) |
| `cargo test --lib -- contexts::agent_runtime`(`-j 3`) | PASSED | 0 | 1379 passed; 1 ignored |
| `cargo test --lib -- contexts::tooling contexts::sessions contexts::permissions` | PASSED | 0 | 1963 passed |
| `cargo test --lib -- --skip contexts::agent_runtime --skip contexts::tooling --skip contexts::sessions --skip contexts::permissions` | PASSED | 0 | 3342 passed; 12 ignored(568 s,贴近单条命令 590 s 上限) |
| `cargo test -p vanehub-ai --test evidence_bridge_architecture --test log_repair_boundaries --test mcp_fixture_contracts --test mcp_relay_provider_invocations --test remote_workspace_ssh --test session_log_index_architecture --test session_shell_architecture` | PASSED | 0 | 12 + 9 + 3 + 3 + 15 + 11 + 8 |
| `cargo test --workspace --tests --exclude vanehub-ai` | PASSED | 0 | `vanehub-permission-hook` 25 passed |
| `cargo test --workspace`(等价拆分) | PASSED | 0 | 以上分片合计即完整工作区测试;整条命令因 OOM 看门狗与 590 s 上限无法单次跑完,与 `extend-cli-providers-with-acp` 的做法一致 |
| `npm run lint:ci` | PASSED | 0 | 桌面层结束后顺序执行(2026-09-07 00:45 UTC 起) |
| `npm run test` | PASSED | 0 | 475 files / 2935 tests |
| `npm run build` | PASSED | 0 | |
| `npm run architecture:check` | PASSED | 0 | |
| `npm run test:coverage` | PASSED | 0 | 2935 passed,覆盖率门槛通过 |
| `npm run docs:check` | PASSED | 0 | |
| `openspec validate --specs --strict` | PASSED | 0 | |
| `openspec validate harden-sqlite-write-transactions --strict` | PASSED | 0 | 实施后复跑 |

## 桌面层(Linux)

| 层 | 结果 | 说明 |
| --- | --- | --- |
| `npm run test:desktop:build` | PASSED | 2026-09-07 00:2x UTC,含本变更的全部原生改动 |
| desktop-smoke | PASSED(25/25 spec,0 次重试,8 min 19 s) | 证据 `2026-09-07T00-31-34-354Z-fb26636d`。WDIO 输出无 `RETRYING` / `database is locked` / `Error: storage`;原生日志无 `storage error`(修复前同一层的 `2026-09-06T18-11-56-958Z-b76f4b9c` 有 2 次 spec 重试和 4 条 retention storage error) |
| desktop-skills | PASSED(1/1) | `2026-09-07T00-39-57-960Z-11e4ec6a` |
| desktop-settings-persistence | PASSED(2/2) | `2026-09-07T00-40-14-738Z-a589695b` |
| desktop-scheduled-tasks | PASSED(1/1) | `2026-09-07T00-40-43-621Z-949a7a5f` |

Windows / macOS:NOT RUN(本机无对应环境,须由 CI 各平台 runner 执行)。

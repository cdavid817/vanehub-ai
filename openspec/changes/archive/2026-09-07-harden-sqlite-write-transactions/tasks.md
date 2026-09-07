# Tasks

已按 2026-09-07 的实际完成状态勾选;验证项的记录见 verification/results.md。

## 1. 审计与基线

- [x] 1.1 以 [references/transaction-audit.md](references/transaction-audit.md) 为起点,逐个打开生产代码中的 `transaction()` / `unchecked_transaction()` 调用点,人工确认分类(先读后写 / 写优先 / 纯读快照 / 启动期迁移),修正启发式误判并更新清单。
- [x] 1.2 对每个"先读后写"点记录事务内读取的范围;凡是大范围扫描(retention、capacity enforcement、reconcile)标注是否需要把只读部分移出事务。
- [x] 1.3 在 `src-tauri/ARCHITECTURE.md` 的 SQLite 行记录"写事务必须 Immediate"的决定及本变更引用。

## 2. 平台入口与守护

- [x] 2.1 在 `platform::database` 增加 `SqliteWriteTransaction` 扩展 trait(`write_transaction` / `write_transaction_unchecked`),固定 `TransactionBehavior::Immediate`,返回 `rusqlite::Result` 以保留各 context 既有的错误映射(design.md 已按此更新);补单测覆盖两种形态。
- [x] 2.2 增加真实争用回归测试:两条池连接,deferred 先读后写断言得到 busy,`write_transaction` 断言等待后成功。
- [x] 2.3 在 `src-tauri/tests/architecture.rs` 增加 fitness 测试:非测试源码中对 `transaction`、`unchecked_transaction`、`transaction_with_behavior` 的调用必须命中允许清单;守护与改造同批落地,清单最终只含三条纯读快照与两项目录/文件豁免。

## 3. 改造事务点

- [x] 3.1 已在真实桌面进程中失败过的四处优先:`tooling/prompt_hooks::publish_draft`、`work_board::move_item`,以及复核 `tooling/skill_tools::save_trust`、`tooling/cli_parameters::{replace_if_revision, reset_if_revision}` 改为经由 `write_transaction`;各补一条同结构单测。
- [x] 3.2 其余确认的先读后写点(`agent_runtime` 注册表删除与 OnePiece profile、`execution_observability::maintain_retention`、`sessions::acknowledge_recovery`、`skill_evolution_*` 的 rebuild/notification/curation/supersession/retention、`tooling/skills::reconcile_workspace_aliases`、`workspaces::enforce_capacity`、`skill_evolution_generation::regenerate`)改为写事务;按 1.2 的结论处理只读扫描。
- [x] 3.3 写优先的事务点统一改为 `write_transaction`;每改一个文件即从基线清单移除对应条目。
- [x] 3.4 确认为纯读快照的保留 deferred,在允许清单中写明理由;不需要事务的纯读改为直接查询。
- [x] 3.5 `platform/database/migrations` 的启动期迁移事务登记为允许项,不改动。

## 4. 验证

- [x] 4.1 运行 AGENTS.md「校验命令」全部命令(`npm run lint:ci`、`npm run test`、`npm run build`、`cargo fmt --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`npm run native:panic:check`、`cargo test --workspace`、`openspec validate --specs --strict`),以及 `npm run architecture:check`、`npm run test:coverage`。
- [x] 4.2 `npm run test:desktop:build` 后运行 `npm run test:desktop:smoke` 与 `skills`、`settings-persistence`、`scheduled-tasks` 层;要求全量通过且原生日志与 WDIO 输出中首轮不再出现 `database is locked` / `storage`;按平台分别记录 PASSED / FAILED / BLOCKED / NOT RUN。(Linux 四层 PASSED;Windows / macOS NOT RUN)
- [x] 4.3 `openspec validate harden-sqlite-write-transactions --strict`;在本目录 `verification/results.md` 记录全部命令、退出码与证据目录。

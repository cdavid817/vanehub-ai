# Harden SQLite write transactions against lock-upgrade failures

## Why

原生运行时的 SQLite 连接统一配置为 WAL + 5 秒 `busy_timeout`,预期是"争用时等待而不是立刻失败"。但 rusqlite 的 `Connection::transaction()` 默认是 **deferred** 事务:事务从第一条 `SELECT` 起以读快照身份持有共享锁,直到第一条写语句才尝试升级为写锁。在 WAL 下,如果读快照建立之后任何其他连接已经提交,SQLite 会立刻返回 `SQLITE_BUSY_SNAPSHOT`,**`busy_timeout` 对这种升级完全不生效**。结果是:一次合法的"先读校验、再写入"的用例,只要与任何后台写入(Skill registry refresh、执行观测 retention、scheduled maintenance、终端用量轮询、IM 去重清理)撞上同一个时间窗,就以 `database is locked` / `storage` 失败,而且失败不带任何"可重试"语义直接抛给用户。

这不是理论风险。2026-09-05 的 `add-session-worktree-cleanup` 桌面层首次踩到(`sessions/infrastructure/transactions.rs`,已局部修复);2026-09-06 `extend-cli-providers-with-acp` 的全量 desktop smoke 又连续暴露三处并全部复现于真实桌面进程:`skill_tools::save_trust`(Skill 工具信任决定丢失)、`cli_parameters::{replace_if_revision, reset_if_revision}`(CLI 参数保存/重置失败),修复后同一轮里 `prompt_hooks::publish_draft` 与 `work_board::move_item` 仍各以 `database is locked` 失败一次、靠 spec 级重试通过。每一处都是同一个缺陷的又一个实例,而全仓还有 123 处 `transaction()` 调用(生产代码 99 处、测试 24 处),生产代码中按启发式至少 25 处是先读后写。逐个等它在用户手里出错再修,不如一次按规则收口。

## What Changes

- 在 `native-runtime-architecture` 规范中新增要求:任何会写入的 SQLite 事务必须在开始时就取得写锁(`BEGIN IMMEDIATE`),deferred 事务只允许用于纯读快照;读后写的争用必须表现为等待 `busy_timeout`,而不是即时失败。
- 在 `platform::database` 提供唯一的写事务入口(`write_transaction`),封装 `TransactionBehavior::Immediate` 与统一的错误映射;各 context 的 infrastructure 通过它开启写事务,不再各自调用 `transaction()`。
- 审计并改造全部生产代码中的 deferred 事务:先读后写的必须改为写事务;写优先的同样改为写事务(统一规则,取锁时机只提前到事务开头,没有额外代价);确认为纯读的保留 deferred 并在审计清单中登记理由。测试代码中的事务不在改造范围。
- 增加架构 fitness 测试:扫描 `src-tauri/src` 非测试源码,凡是直接调用 `transaction()` / `unchecked_transaction()` 的位置必须出现在显式的"纯读快照"允许清单中,否则构建失败;新增写路径无法再悄悄回到 deferred。
- 增加一个真实并发的回归测试:两条池连接,一条持有已提交的写,另一条先读后写,断言写事务等待并成功,而不是返回 busy;为已知失败的四个仓储各补一条同形态的单测。
- 不改变任何表结构、迁移顺序、事务边界内的 SQL 语义或 API 形状;不引入重试循环掩盖失败。

## Capabilities

### New Capabilities

- 无。

### Modified Capabilities

- `native-runtime-architecture`:新增"写事务在开始时取得写锁"的要求及其争用、纯读、机械守护三个场景;既有 requirement 与 scenario 标题不变。

## Impact

### Native

- `src-tauri/src/platform/database/mod.rs`:新增写事务入口与配套单测。
- 约 50 个 infrastructure 文件、99 个生产事务点(清单见 [references/transaction-audit.md](references/transaction-audit.md),按启发式分类,任务 1 人工确认):`sessions`、`tooling/{skills,skill_tools,prompt_hooks,cli,cli_parameters,extensions}`、`agent_runtime`、`execution_observability`、`operations`、`communications`、`workspaces`、`work_board`、`retrieval`、`permissions`、`desktop`、`ssh_connections`、`personalization`、`code_intelligence`、`artifacts`、`skill_evolution_*`。
- `src-tauri/tests/architecture.rs`:新增 fitness 测试;`src-tauri/ARCHITECTURE.md` 的 SQLite 一行记录该决定。

### Frontend / Web

- 无。前端与 Web/mock adapter 不感知事务行为。

### Compatibility and risk

- 行为差异只有一种:原本会以 `database is locked` 即时失败的写入,现在最多等待 `busy_timeout`(5 秒)后成功或以同一错误失败。长事务持锁时间不变,但取锁提前到事务开头,长时间"先读再写"的事务(如 retention 扫描)会更早阻塞其他写者,任务 1 需逐个确认没有把大范围只读扫描包进写事务。
- 不包含:改 `busy_timeout` 值、改 WAL/`synchronous` 设置、引入连接级串行化、给命令层加重试、改动迁移事务(启动期单线程)。

## Delivery and Acceptance

顺序:平台入口与 fitness 测试(先让守护落地并记录当前违例基线)→ 已确认的先读后写点 → 其余写事务 → 纯读清单确认与文档。验收以 AGENTS.md 全量校验命令、并发回归测试,以及 `npm run test:desktop:smoke` 全量通过且首轮无 `database is locked` 为准;桌面层结果按平台分别记录,不外推。任务见 [tasks.md](tasks.md)。

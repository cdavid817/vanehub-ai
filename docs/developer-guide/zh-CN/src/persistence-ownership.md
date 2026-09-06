# 持久化所有权

哪个上下文拥有哪张表、迁移怎么编号与执行，以及数据库层的关键常量。

日志落盘、脱敏与链路关联见[统一日志](unified-logging.md)。

## SQLite 所有权

SQLite 只能从 Rust 基础设施访问。迁移有一个全局顺序,但每个 schema 与 repository 都归属于某个 bounded context。外键引用并不授予一个 context 直接查询另一个 context 表的权限。

迁移变更要求:

- 一个带版本的迁移;
- clean-database 与升级路径的覆盖;
- 显式的行到领域映射;
- 与当前 fixture 兼容;
- 在生产 command 边界上不得使用 `unwrap()` 或 `expect()`。

## SQLite 所有权与迁移

数据库是应用拥有的单一 SQLite 文件。连接、迁移编排与种子注册都集中在 `src-tauri/src/platform/database/mod.rs`，任何 context 都不得绕过 pool 自建连接或自管 schema。

- **单一数据库文件** `vanehub.sqlite`，启用 `journal_mode=WAL`（多读一写）、`foreign_keys=ON`、`synchronous=FULL`（在每个恢复关键提交点同步 WAL）。
- **连接池** 上限 `MAX_POOL_SIZE = 12`，`busy_timeout = 5s`，`CONNECTION_TIMEOUT = 5s`；池大小接近 Tauri command worker 线程数，WAL 让多读者不被写者阻塞。
- **顺序迁移** 在 pool 共享前于一个独占连接上跑一次，`schema_migrations` 表为每条迁移记账（版本号 + 名称）。
- **`EXPECTED_MIGRATIONS` 是迁移序列的真源**，启动后密度检查与 `migration_sequence_matches_expected` 测试都会比照它。新增迁移必须追加在序列尾部，禁止插入或重排——版本号是跨分支分配的，重排会让已应用旧号的检出全部失效。
- **种子注册** `seed_registry` 在迁移完成后于同一独占连接上执行一次。
- **限界上下文分区** 各 context 拥有各自的表（靠迁移分区写入），外键引用不授予一个 context 直接查询另一个 context 表的权限。

```mermaid
flowchart TD
    AppStart([应用启动]) --> NewPool[创建连接池<br/>WAL / foreign_keys=ON / synchronous=FULL<br/>pool ≤ 12, busy_timeout=5s]
    NewPool --> Exclusive[取一个独占连接]
    Exclusive --> Migrate[migrate conn]
    Migrate --> SchemaMig[CREATE TABLE schema_migrations<br/>若不存在]
    SchemaMig --> ApplySeq[按 EXPECTED_MIGRATIONS 顺序应用迁移]
    ApplySeq --> Book[每条迁移写入<br/>schema_migrations 版本+名称]
    Book --> Seed[seed_registry conn]
    Seed --> SharePool[pool 共享给各 context]
    SharePool --> Ctx1[sessions 限界上下文<br/>自有表]
    SharePool --> Ctx2[agent_runtime 限界上下文<br/>自有表]
    SharePool --> Ctx3[code_intelligence 限界上下文<br/>自有表]
    Ctx1 -.外键不授予跨域查询.-> Ctx2
```

## 数据库常量与迁移

### 数据库常量

- `DATABASE_FILE_NAME="vanehub.sqlite"` —— 单一数据库文件;
- `MAX_POOL_SIZE=12` —— 连接池上限,接近 Tauri command worker 线程数;
- `busy_timeout=5s` —— 写者阻塞时读者等待上限;
- `CONNECTION_TIMEOUT=5s` —— 取连接的超时;
- `journal_mode=WAL` —— 多读一写;
- `foreign_keys=ON` —— 外键约束启用;
- `synchronous=FULL` —— 在每个恢复关键提交点同步 WAL。

### 迁移

`EXPECTED_MIGRATIONS`(`src-tauri/src/platform/database/migrations/mod.rs`)是迁移序列的真源;启动后密度检查与 `migration_sequence_matches_expected` 测试都会比照它。新增迁移必须追加在序列尾部,禁止插入或重排。本章刻意不写迁移条数——版本号跨并行分支分配,任何写死的数字在第二个分支合入时就已过时。`schema_migrations(version, name, applied_at)` 表为每条迁移记账。`seed_registry` 在迁移完成后于同一独占连接上执行一次。

## 会话删除 journal 与受管理 worktree

删除会话是唯一会同时触碰 Git 与 SQLite 的用例，两者不在一个事务里，所以它的状态由独立的 journal 表承载，而不是从日志文本推断。

| 表 | 所属 context | 说明 |
| --- | --- | --- |
| `managed_worktrees` | `workspaces` | 由本应用创建的 worktree 资源记录：来源（`ordinary_session`/`loop`/`subagent`/`external`）、来源证据等级（`provisioning`/`verified`/`legacy_verified`/`legacy_unverified`/`external`）、Git 身份（canonical root、git dir、common dir、分支、HEAD、文件系统身份）、状态与修订号。意图在 `git worktree add` 之前写入，Git 成功后再补身份 |
| `managed_worktree_sessions` | `workspaces` | worktree 与会话的绑定。**没有指向 `sessions` 的外键**：会话删除后资源记录必须继续存在 |
| `workspace_use_gates` | `workspaces` | 清理期间对目录的独占门禁，记录持有实例与操作。实例是否存活由 `platform::instance_lease` 的 OS 文件锁证明，不靠 TTL 猜测 |
| `session_deletion_operations` / `session_deletion_groups` | `sessions` | 每次删除请求的 journal：`request_id` 唯一并绑定规范化请求 hash（幂等），分组按真实 worktree 身份划分，逐组记录 `worktree_effect`（`not_requested`/`retained`/`remove_started`/`removed`/`removal_unknown`）与 `db_effect`，以及授权快照、执行快照和回执 |
| `session_deletion_claims` | `sessions` | 删除期间对会话的独占 claim。`sessions` 服务在发送消息、开始生成、改席位、归档/恢复前检查它；`workspaces` 通过 bootstrap 注入的 `WorkspaceExecutionAdmissionPort` 在开 Shell 前检查它。同样**没有指向 `sessions` 的外键** |

不变量：

- 先写 journal，再停止会话；先持久化 `remove_started` 与身份快照，再执行唯一允许的非 force `git worktree remove`；先观察目录与登记确实消失，再在一个事务里删除该组会话及其级联数据并清空匹配的活动会话。
- Git 成功而会话事务失败记录为 `finalize_pending`，claim 继续持有；重启后 `reconcile_pending_deletions` 只重新观察并完成数据库收尾，**永不重放 Git 删除**。目录既不完整也未确认消失时进入 `needs_attention`，不 prune、不递归删除。
- 旧的 `delete_session` 命令改为走同一协调器的 keep 路径，等待真实完成，并遵守 claim。

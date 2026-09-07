# Design

## Context

`platform::database::NativeDatabase` 用 r2d2 连接池,每条物理连接初始化时设置 `busy_timeout = 5 s`、`journal_mode = WAL`、`synchronous = FULL`。多个后台任务(Skill registry refresh、execution observability retention、scheduled maintenance、终端用量轮询、IM 去重清理)与命令处理线程共享同一个数据库文件。

SQLite 的锁语义决定了两类事务在争用下的表现完全不同:

| 事务形态 | 争用时的表现 |
| --- | --- |
| `BEGIN IMMEDIATE`(写事务) | 开始时申请 RESERVED 锁;若被占用,按 `busy_timeout` 等待 |
| `BEGIN`(deferred)且第一条语句是写 | 第一条写语句申请 RESERVED 锁;同样按 `busy_timeout` 等待 |
| `BEGIN`(deferred)先读后写 | 读语句建立 WAL 快照;之后的写语句若发现快照已过期,立即返回 `SQLITE_BUSY_SNAPSHOT`,`busy_timeout` 不生效 |

第三种正是仓库里"先校验 revision / 状态,再写入"这类最常见的仓储形态。

## Decisions

### 规则:会写的事务一律 Immediate,deferred 只给纯读

不区分"写优先"与"先读后写":两者都改为 Immediate。理由是写优先的事务改成 Immediate 只把取锁时机从第一条写语句提前到 `BEGIN`,中间通常没有任何语句,没有可观测的代价;而让维护者在每个事务点判断"第一条语句是不是读"是这个缺陷反复出现的根源。规则简单到可以被机器检查。

纯读快照(多条 SELECT 需要一致视图)保留 deferred,但必须登记在 fitness 测试的允许清单里并写明理由。当前普查里只有极少数候选(如 `work_board::reconcile` 的读取部分),多数"只读"其实不需要事务。

### 平台入口:`platform::database::SqliteWriteTransaction`

```rust
pub(crate) trait SqliteWriteTransaction {
    fn write_transaction(&mut self) -> rusqlite::Result<Transaction<'_>>;
    fn write_transaction_unchecked(&self) -> rusqlite::Result<Transaction<'_>>;
}
impl SqliteWriteTransaction for Connection { /* BEGIN IMMEDIATE */ }
```

- 以扩展 trait 而非自由函数提供:调用点从 `conn.transaction()` 改为 `conn.write_transaction()`,`PooledSqlite` 经 auto-deref 直接可用,diff 只有方法名与一行 `use`。
- 返回 `rusqlite::Result`,不映射为 `DatabaseError`:各 context 已有的 `.map_err(storage)` 链保持原样,改造不触碰任何错误类型。
- `write_transaction_unchecked` 对应 `Transaction::new_unchecked(.., Immediate)`,给持有 `&Connection` 的仓储(`skill_evolution_generation` retention 等)使用;调用方负责保证该连接上没有其他事务。
- 不提供 deferred 的对应入口,保持"要开写事务只有一种写法"。
- 不引入闭包式 `with_write_transaction(|tx| ...)`:现有代码普遍在事务中间提前 `return Err(...)` 依赖 `Transaction` 的 drop 回滚,闭包式改造会放大 diff 且改变错误流。

### 机械守护:架构 fitness 测试

在 `src-tauri/tests/architecture.rs` 增加一条测试:遍历 `src-tauri/src` 的非测试 Rust 源(复用现有 `is_test_source` 与 `is_test_only` 判定,跳过 `#[cfg(test)]` 模块与 `*_tests.rs`),用 `syn` 找出对 `transaction` / `unchecked_transaction` / `transaction_with_behavior` 的方法调用以及 `Transaction::new(_unchecked)` 构造;每个命中必须出现在测试内的允许清单(`file`, `fn`, `reason`)中,否则失败并打印位置。`platform/database/` 整个目录豁免(入口自身与启动期迁移),`cli_parameters/legacy_baseline.rs` 因仅在 `cfg(test)` 下编译而豁免;其余源码直接调用 `transaction_with_behavior` 同样视为违例,防止绕过入口。

实施时守护与改造在同一批落地,清单最终只有三条纯读快照:`sessions::read_terminal_evidence`、`skill_evolution_curation::{review_binding, preview_binding}`(后两者原本就显式写成 `Deferred`)。

### 回归测试:真实争用而不是 mock

`platform::database` 增加一条测试:同一个 `NativeDatabase` 取两条池连接,连接 A 先 `SELECT` 再在连接 B 提交一次写入,然后 A 执行写入。deferred 形态断言得到 `SQLITE_BUSY`(证明测试能复现缺陷),`write_transaction` 形态断言成功。这条测试直接对应规范里的争用场景。四个已知失败仓储(`skill_tools::save_trust`、`cli_parameters::replace_if_revision`、`prompt_hooks::publish_draft`、`work_board::move_item`)各补一条同结构的单测。

### 迁移事务不动

`platform/database/migrations` 里的 `unchecked_transaction` 在启动期单线程执行,在任何命令或后台任务开始前完成,没有争用;它们在允许清单中登记为"启动期迁移"。

## Alternatives considered

- **在命令层对 `database is locked` 重试**:掩盖根因,且对非幂等写入不安全;拒绝。
- **单一写连接串行化全部写入**:改变整个持久化拓扑,与现有连接池和后台任务模型冲突,收益不比 Immediate 多;拒绝。
- **只修已确认的 24 处先读后写**:留下 50 多处需要人工判断的写优先事务,下一次有人在事务开头加一条校验 `SELECT` 就会复发;拒绝。

## Risks

- 长扫描包进写事务:retention 类任务先做大范围 `SELECT` 再 `DELETE`,改为 Immediate 后扫描期间就持有写锁。任务 1 逐个确认这类事务的读取部分是否需要留在事务内,必要时把只读扫描移到事务外或改用单条 `DELETE ... WHERE` 语句。
- 已有单测里对 deferred 行为的隐式依赖(例如测试中用另一条连接观察事务内部状态):以 `cargo test --workspace` 全量结果为准逐个处理,不删测试。

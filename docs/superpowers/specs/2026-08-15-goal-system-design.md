# 目标系统设计

日期：2026-08-15
状态：设计已确认，待起 OpenSpec proposal

## 目标

把 `goal` 从当前散落在 Loop 与 Plan 内部的私有字段，提升为独立的顶层实体。目标之下可挂载 Plan、Loop、WorkItem、Session 四类执行体，目标自身有状态机与人工验收流程，并能跨会话追踪进度。

第一期不改动任何执行体的既有行为：Loop 与 Plan 的 `goal` 文本字段原样保留，关联关系单向地由目标侧持有。

## 现状

`goal` 目前有两处彼此独立的实现，都没有对应的 OpenSpec 规范。

### Loop 引擎

`goal` 与 `acceptance_criteria: Vec<String>` 实际落在 `LoopDefinitionInput`（`agent_runtime/domain/loop_engineering.rs:272-273`）上，`LoopDefinition`（`:286`）是它的 newtype 包装，在 `new()` 中强制校验两者非空。运行期这两者是不可变快照，`loop_verifier.rs:199` 把它们与确定性检查证据、有界 git diff 一起交给独立 verifier 判定。`LoopTerminalReason` 含 `GoalMet`，而 `loop_decision.rs:54` 明确写着目标达成仍需人工接受。

### Plan 中心

`goal` 挂在 `PlanVersion` 上而非 `Plan`（`task_orchestration/domain/model.rs:444`），即 Plan 的目标随版本走。用户填目标后由 OnePiece 生成计划草稿，经人工审阅进入 `PlanStatus::Approved`。

### 工作看板

`WorkItem` 已有 `WorkItemSourceLink` 机制（`work_board/models.rs:18`），看板项能链到 Plan。`stage` 是自由字符串，取值由前端 `src/types/work-board.ts:1` 定义为 `["inbox", "planned", "in_progress", "review", "done"]`，Rust 侧在 `work_board/api.rs:29-30` 用 `STAGES` 白名单校验。

## 架构选择

目标要同时统辖分散在 `task_orchestration`、`agent_runtime`、`work_board`、`sessions` 四个 context 的对象。

采用的方案：**新建 `goals` bounded context，关联关系由目标侧独占**。四个执行体的表结构一列不改，`goal_links` 是唯一的关联真源。

放弃的方案与理由：

- 四个执行体各加 `goal_id` 列：正向查询更快，但要做四处 schema 迁移、四个 context 都要感知 Goal。考虑到本仓库所有 worktree 共享同一个 SQLite 文件，四个迁移版本号比一个更容易与其他分支撞车。
- 把目标做成 `WorkItem` 的一种类型：工作量最小，但 `WorkItem` 是扁平待办，撑不起独立状态机与跨执行体聚合，与"顶层一等实体"的定位冲突。

## 领域模型

```
Goal（聚合根）
  id, title, description, acceptance_notes,
  status, project_path?, created_at, updated_at

GoalLink
  goal_id, target_kind, target_id, linked_at
  UNIQUE(goal_id, target_kind, target_id)

GoalLinkTarget = Plan | Loop | WorkItem | Session
```

`acceptance_notes` 是给人看的验收依据，不参与机器判定。目标不引入自己的验证命令——Loop 与 Plan 各自已有完整的证据链，目标再加一套会与子项验收形成双重判定冲突。

### 状态

持久化状态只有四个：`Draft`、`Active`、`Achieved`、`Abandoned`。

`AwaitingAcceptance` **不落库**，每次查询现算。这不是风格选择，而是拉模式的硬性要求：系统不订阅子项状态变更事件，若把待验收写进数据库，子项被重开后目标会永久停在错误状态。派生态每次重算，天然自愈。

人工迁移规则：

- `Draft` → `Active`
- `Active` → `Achieved`，仅当派生态为 `AwaitingAcceptance` 时允许
- `Achieved` → `Active`，重开
- 任意状态 → `Abandoned`；`Abandoned` → `Active`

## 达成推导

进度采用拉模式：打开目标时现查四类子项状态并聚合，不引入领域事件。

每个链接经 probe 归为三态之一：`Terminal`、`Active`、`Unresolvable`（目标对象已删除或查询失败）。

派生规则：`status == Active` 且可推导子项数大于零且全部为 `Terminal` 时，呈现为 `AwaitingAcceptance`；否则呈现 `status` 本身。

### 各执行体的终态判定

**Loop**：直接复用既有的 `LoopRunStatus::is_terminal()`（`loop_engineering.rs:47`），即 `Succeeded | Failed | Cancelled`。

**Plan**：终态仅为 `PlanRunStatus::{Completed, Cancelled}`。**`Failed` 不是终态**——`can_transition_to` 中存在 `(Failed, Running)` 这条边（`model.rs:188`），失败的 Plan run 可以重跑。Loop 与 Plan 在这一点上语义不对称，两个 probe 必须分开实现，不能共用一份终态定义。

Plan 链接指向 Plan 而非某次 run，终态由其最新一次 `PlanRun` 决定；尚无 run 时视为 `Active`；`PlanStatus::Archived` 视为 `Terminal`（用户已主动搁置）。

**WorkItem**：`stage == "done"` 为终态。`archived` 为真时同样视为终态。

**Session**：永不参与推导。会话没有"完成"语义，且数量大、生命周期短。关联仅供人工浏览，且只能由用户显式建立，不做自动挂载。

### 两条易错规则

以下两条必须在测试中钉死，它们是本设计最容易被实现者写反的地方：

1. **停在 `AwaitingAcceptance` 的 Loop 或 Plan 不算终态**。既有的 `is_active()` 明确把 `AwaitingAcceptance` 归为活跃（`loop_engineering.rs:40-45`）。子项自己还等着人验收，目标就不能宣称待验收。
2. **`Unresolvable` 子项不计入分母**，但需在 UI 明确标出。否则一个被删掉的 Plan 会让目标永远卡在未完成。

## 分层落点

```
src-tauri/src/contexts/goals/
├─ mod.rs
├─ api.rs
├─ domain/
│  ├─ goal.rs          Goal、GoalStatus、迁移规则、GoalDomainError
│  └─ link.rs          GoalLink、GoalLinkTarget
├─ application/
│  ├─ ports.rs         GoalRepository trait + LinkProgressProbe trait
│  ├─ goal_service.rs  CRUD 与状态迁移
│  └─ progress.rs      纯函数聚合
└─ infrastructure/
   ├─ goal_repository.rs   SQLite 实现
   └─ progress_probes.rs   四个只读 probe
```

这个结构的全部意义在于：`domain/` 与 `application/` 不 import 任何执行体 context。probe 以 trait 形式定义在 `application/ports.rs`，实现落在 `infrastructure/`。这是"零侵入"承诺的兑现点，也让 `progress.rs` 成为可用假 probe 穷举测试的纯函数。

`GoalDomainError` 照 `PlanDomainError`（`model.rs:5`）的既有写法定义。

Tauri commands 落在 `src-tauri/src/commands/goals/`，每命令一个文件：`create_goal`、`update_goal`、`delete_goal`、`list_goals`、`get_goal`、`link_goal_target`、`unlink_goal_target`、`accept_goal`、`reopen_goal`、`abandon_goal`。跨 command 边界的错误一律转为 `Result<T, String>`。

### 前端

- `src/contracts/goal.ts`：类型契约
- `src/services/goal-service.ts`：服务接口
- `src/services/tauri-goal-client.ts` 与 `src/services/web-goal-client.ts`：两个实现必须同步提供，这是 AGENTS.md 的硬约束
- `src/goal-center/`：UI，参照 `src/loop-center/` 的组织方式

组件不得直接调用 `invoke()`。每个文件不超过 300 行——`max-lines` 是 ESLint 硬规则，新文件一律不得进 `eslint.config.js` 的技术债豁免清单。

### 必须同步的 context 清单

新建 `src-tauri/src/contexts/goals/` 的同一个变更里，必须往 `openspec/project.md` 的「### Bounded contexts」表加一行。该表自称是完整映射，且由 `scripts/validate-docs.mjs:301` 强制校验目录与表行严格一一对应，漏改会让 `npm run docs:check` 失败。

同一份规范还写明：只有行为确属无状态持久化的 context 才可省略 `domain/` 与 `application/`（现存例子是 `work_board`）。目标有状态机这一不变量要保护，因此必须采用完整分层。

## 风险点

### 数据库迁移版本号

新增 `goals` 与 `goal_links` 两张表，只需一个迁移。

本仓库所有 worktree 共享同一个 `%APPDATA%\ai.vanehub.app\vanehub.sqlite`。迁移版本号与其他分支撞车会导致应用启动崩溃并报"no such table"，且症状看起来像真实回归而非环境问题。落号前必须先查 `schema_migrations` 表以及其他活跃分支已占用的号段。

### probe 失败降级

单个 probe 抛错不能拖垮整个目标查询。失败的链接降级为 `Unresolvable`，其余子项照常聚合。

### 目标被永久阻塞

失败但可重试的 Plan run 会让目标一直停在未完成。这是符合语义的——工作确实没做完。用户可以重跑、解除关联或放弃目标。UI 需要让阻塞原因一眼可见，而不是只显示一个不动的进度条。

## 测试策略

Rust 侧：

- `domain/goal.rs` 的状态迁移用表驱动测试覆盖全部合法与非法迁移
- `application/progress.rs` 用假 probe 覆盖：无子项、部分终态、全终态、probe 失败、仅有 Session 链接、Loop 停在 `AwaitingAcceptance`、Plan 停在 `Failed`

前端侧：

- `tauri-goal-client` 与 `web-goal-client` 的行为对齐契约测试，参照仓库已有的 `*-adapter-parity.test.ts` 成例
- 组件测试注意三个既有陷阱：jest-dom 匹配器不可用、默认语言不是英文、需要 jsdom 指令

日志走统一日志服务，不新建 feature-local 日志文件，敏感信息落盘前脱敏。

## 后续

按 AGENTS.md，动代码前必须在 `openspec/changes/` 下起 proposal 并通过 `openspec validate --strict`。

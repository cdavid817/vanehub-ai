## Why

「目标」在本产品里已经存在两次，但都是别人的私有字段：`LoopDefinitionInput` 有 `goal` 与 `acceptance_criteria`，`PlanVersion` 也有 `goal`。两者互不相识，各自实现验收判定，且都随宿主对象的生命周期消亡。用户没有任何位置能回答「我这周在推进哪几件事、各自到哪一步了」——一个目标下的 Plan、Loop 与看板项散在三个互不连通的页面里。

现在做的理由是关联面还小：目前只有四类可挂载对象，且 `work_board` 已经用 `WorkItemSourceLink` 验证过「关联关系由一侧独占」这个模式。等执行体类型继续增长再抽取顶层实体，成本只会更高。

## What Changes

- 新增 `goals` bounded context，Goal 为聚合根，持有 `title`、`description`、`acceptance_notes`、`status`、可选 `project_path`
- 新增 `goal_links` 关联表，`(goal_id, target_kind, target_id)` 唯一。`target_kind` 取 `Plan | Loop | WorkItem | Session`
- **不改动任何执行体**：Loop 与 Plan 的 `goal` 文本字段原样保留，四个执行体的表结构一列不动。关联关系单向地由目标侧持有，这是本变更零侵入承诺的全部依据
- 目标持久化状态为 `Draft | Active | Achieved | Abandoned` 四个。**`AwaitingAcceptance` 不落库**，每次查询按子项现算——系统不订阅子项状态事件，落库的待验收会在子项被重开后永久停在错误状态
- 进度采用拉模式：打开目标时现查子项状态并聚合，不引入领域事件，四个执行体 context 无需发布任何东西
- 各执行体的终态判定**必须分开定义**。Loop 复用既有 `LoopRunStatus::is_terminal()`（含 `Failed`）；Plan 的终态仅为 `Completed` 与 `Cancelled`，因为 `can_transition_to` 存在 `(Failed, Running)` 这条边，失败的 Plan run 可以重跑
- Session 关联只能由用户显式建立，且**永不参与达成推导**。会话没有「完成」语义，自动挂载会让目标详情页迅速被短会话淹没
- 新增 Tauri commands 与前端 `goal-service` 服务边界，`tauri-goal-client` 与 `web-goal-client` 两个实现同时提供
- 新增 `src/goal-center/` UI 页面

## Capabilities

### New Capabilities

- `goal-management`: 定义目标实体的生命周期与状态迁移、四类执行体的关联与解除、达成推导规则与各执行体的终态判定、人工验收与重开语义、不可解析子项的降级行为。

### Modified Capabilities

无。本变更不改变任何既有能力的对外行为：执行体的状态机、字段与 API 均不变，关联关系完全由新 context 持有。

## Impact

**运行时范围**：桌面运行时与 Web 运行时**均受影响**。目标数据由 Rust 侧 SQLite 持有，Web 运行时通过 `web-goal-client` 的 mock 适配提供同等接口，两个实现必须保持行为一致。

**适配器边界**：不破坏前后端隔离。React 组件只依赖 `src/services/goal-service.ts`，不直接调用 `invoke()`。跨 command 边界的错误统一转为 `Result<T, String>`。

**受影响代码**：

- 新增 `src-tauri/src/contexts/goals/`（完整分层：`domain/`、`application/`、`infrastructure/`、`api.rs`）
- 新增 `src-tauri/src/commands/goals/`，每命令一个文件
- 新增前端 `src/contracts/goal.ts`、`src/services/goal-service.ts`、`src/services/tauri-goal-client.ts`、`src/services/web-goal-client.ts`、`src/goal-center/`
- 一个新数据库迁移，建 `goals` 与 `goal_links` 两表
- `openspec/project.md` 的「Bounded contexts」表必须在**同一个变更**内加一行。该表自称完整映射，由 `scripts/validate-docs.mjs` 强制与目录严格一一对应，漏改会让 `npm run docs:check` 失败

**只读依赖**：`goals` 的 infrastructure 层需要只读查询 `task_orchestration`、`agent_runtime`、`work_board` 三个 context 的运行状态。probe 以 trait 形式定义在 `application/ports.rs`，`domain/` 与 `application/` 不 import 任何执行体 context。

**风险**：本仓库所有 worktree 共享同一个 `%APPDATA%\ai.vanehub.app\vanehub.sqlite`。新迁移的版本号与其他活跃分支撞车会导致启动崩溃并报「no such table」，症状酷似真实回归。落号前必须先查 `schema_migrations` 与其他分支已占用的号段。

**完整设计**：`docs/superpowers/specs/2026-08-15-goal-system-design.md`。

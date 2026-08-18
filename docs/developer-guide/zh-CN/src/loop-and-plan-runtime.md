# Loop 与 Plan 运行时

VaneHub 为自主工作运行了两个持久化的 native 执行运行时:**Loop** 运行时(基于目标 + 验收准则对一个 Git 项目反复迭代)与 **Plan** 运行时(感知拓扑的子任务调度)。两者都把状态持久化到 SQLite,并把持久化状态视为权威,而非内存中的调度器状态。面向用户的 Loop/Plan 工作流在用户指南中;本章覆盖 native 设计。

## Loop 运行时

一个 Loop 定义在持久化时附带稳定的 id、名称、启用状态、本地 Git 项目路径、基线分支、目标、验收准则、允许与受保护的路径、稳定的 Worker 与 Verifier Agent id、结构化验证命令、停止限制、版本与时间戳。Loop 定义保留**稳定的 Agent id**,而不是匹配显示名称。

第一阶段的范围是有约束的:针对非 Git 项目、远程工作区、缺失 Agent、不安全的路径范围或无效限制的定义,会被拒绝,且不会启动 Agent 或创建 worktree。Worker 与 Verifier 角色既接受 CLI 启动的 Agent,也接受启用了 tool-use 信任的 API Agent;未启用 tool-use 信任的 API Agent 会被拒绝。

## Plan 运行时

一个专用的任务编排边界持久化 `PlanRun`、`SubTaskRun`、`SubTaskAttempt`、验证证据、控制请求与关联记录。在批准一个有效的 Plan 版本时,运行时会在单个一致操作中创建一个 `PlanRun` 快照,并为每个快照中的 SubTask 创建一个 pending 的 `SubTaskRun`。试图认领同一个就绪 SubTask 的重叠调度器 tick,会被一个事务性的 compare-and-set 转换串行化——最多只创建一次调度尝试。

### 确定性的、感知拓扑的串行调度

调度器只派发其依赖已成功的 SubTask,按拓扑秩、Plan 序号与稳定的 SubTask ID 对合格工作进行排序,并在本基础版本中每个 `PlanRun` 同时最多只运行一个 SubTask 尝试。前驱尚未达到已验证成功的 pending SubTask 不会被派发。当多个独立的 SubTask 同时合格时,只派发确定性的第一个。一个失败的必需 SubTask 只会阻塞其传递性后裔,不会阻塞独立分支。

## 设计所在

本章用于为贡献者定向。权威的需求位于规范中。

- [openspec/specs/loop-engineering-runtime](../../../../openspec/specs/loop-engineering-runtime/spec.md) — 持久化的 Loop 定义与 Worker/Verifier 信任契约。
- [openspec/specs/plan-execution-runtime](../../../../openspec/specs/plan-execution-runtime/spec.md) — 持久化执行聚合与串行调度器。
- [openspec/specs/plan-management](../../../../openspec/specs/plan-management/spec.md) — Plan 定义生命周期。

Loop 与 Plan 执行位于 `agent_runtime` bounded context 中;见 [Native bounded context](native-contexts.md)。

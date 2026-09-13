# Change: Enforce Loop execution scope and fail-closed permission delivery

## Why

当前 Loop 保存了 `allowed_paths` / `protected_paths`，但启动权威校验与 UI readiness 不一致，Worker/Verifier 会话没有可信运行范围绑定，路径主要进入提示词。验证命令的 cwd 和程序白名单不能阻止脚本修改范围外文件，人工 accept 也没有范围证据门禁。

另一个明确缺陷在 ACP `handle_read`：只有 `Deny` 被拒绝，`Ask` 会继续读取；而权限存储异常恰好会降级为 `Ask`。因此用户配置不能稳定地转化为执行结果。代码证据和接口清单见 design.md。

## What Changes

- 定义可测试的工作区变更路径语义，采用组件级匹配和实际文件系统身份校验；保护范围是不可被模板、grant 或批准扩大之运行上限。
- 启动与 readiness 共享原生校验器；冻结定义、模式及执行能力证据，worktree 准备后绑定真实根身份，绑定成功之前禁止 Agent/验证进程副作用。
- Native 工具、ACP 代理文件操作、Claude 映射 hook、验证进程和嵌套工具均传递可信范围上下文；未知或不完整的上下文拒绝执行。
- 增加无子进程的内置 `patch-whitespace` 验证类型，为托管 Worker + 原生检查 + 只读 Verifier 提供可跑通的严格模式完整路径；保留原 process 类型，不自动替换用户必需检查。
- 修复所有 ACP 会话中的 `file.read` 三态处理，复用已有单赢家、提交后交付的审批流程，补齐取消、过期、重启和策略故障分支。
- 区分“执行前完整覆盖”“仅托管工具覆盖”“仅产物检查”“未知/不支持”；默认模式只接受完整覆盖。可选产物审计模式必须逐次手动确认，始终披露未受控通道，不能成为自动降级路径。
- 在 Worker、验证命令、Verifier 之后及最终接受前建立范围证据门禁；发现违规保留证据并失败，无法证明则暂停，不能静默接受。
- 同步 Tauri/Web 合约、迁移、恢复、双语 UI 和回归用例。Web 模拟明确标注，不能提供桌面强制约束证明。

## Capabilities

### New Capabilities

- `loop-execution-scope`: 工作区变更范围、可信绑定、真实能力证明、执行前检查和完整产物证据。

### Modified Capabilities

- `loop-engineering-runtime`: 保存/启动/执行/验证/决定/接受/恢复中的范围门禁。
- `permissions-core`: 范围上限先于普通策略准入、评估健康状态及 ACP 读取的 fail-closed 行为。
- `permissions-approval`: 范围与资源绑定的批准、ACP 读取延迟交付和再校验。
- `loop-management-ui`: readiness 与执行覆盖程度分开展示、模式确认和阻断证据。
- `cli-agent-permission-launch-flags`: 启动参数与路径隔离能力分离，按真实执行链评估而非按 Agent 名称推断。

## Impact

桌面 Rust 负责文件系统身份、SQLite、策略、启动、进程和证据；React 只调用服务；Tauri adapter 转发类型化请求；Web/mock 实现相同请求和状态，使用模拟证据。保持现有 DDD、稳定 Agent ID、统一权限入口、审批存储事务与日志体系，不引入新状态管理库或重型编排框架。

受影响上下文：agent_runtime、permissions、sessions、Loop 合约与适配器、数据库迁移、Loop UI、本地化及相关测试。详细文件映射在 design.md。

**兼容性变化：** 新定义默认要求执行前约束；旧定义没有范围版本/模式时保留为 `legacy-unverified`，需要用户显式编辑确认后才可启动。旧的非终态运行不能自动补出授权绑定后恢复。历史终态结果保留历史事实，不标成已强制校验。CLI 的“支持交互”仍是角色基本资格，但不再代表能够满足某次 Loop 的执行范围。

`acp-permission-bridge` 当前仅存在于活动变更 `extend-cli-providers-with-acp`，尚无同名主规范。本变更不对不存在的主规范做 MODIFIED，不重复引入该能力；ACP 修复要求落在 `permissions-core` / `permissions-approval`。实施时协调活动变更的接口，禁止改写它或归档目录来通过校验。

## Non-Goals

- 不实现一套新的跨平台操作系统沙箱，也不把 worktree、cwd、启动参数或 ACP 支持宣称为沙箱。
- `allowed_paths` / `protected_paths` 约束工作区变更，不重新定义读取保密性、网络出口或远程执行策略；这些继续使用各自已有权限。ACP `file.read` 的 Ask 修复适用于普通会话及 Loop。
- 不改造 Memory/Eval、新增 MCP 平台、自动合并、远程发布或自动修复/删除越界文件。

## Delivery Gates

任务全部完成需要：真实 native 严格 Loop 从 start、Worker、必需 verification、只读 Verifier 到 accept 完整跑通，包含允许变更及越界零副作用证据；严格模式下未覆盖通道启动阻断；CLI 兼容模式的真实限制与宿主完整性信任前提展示、逐操作一次确认及产物门禁；ACP 三态读取及故障测试；迁移/恢复/竞态与双适配器一致性；仓库完整质量门禁。只禁用所有 Loop、只验证单个工具、只更新提示词或只验证 Git diff 均不满足交付。

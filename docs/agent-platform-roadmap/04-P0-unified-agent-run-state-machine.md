# P0 — Unified Agent Run State Machine

## 1. 目标

统一当前 Session、Agent generation、Plan、Loop、Goal、Task、operations、Group Chat 等执行生命周期中的状态语义。

不是把所有领域对象合并成一个表，而是建立一个稳定的底层：

> `Run` = 一次可观察、可暂停、可恢复、可取消、可验证的执行实例。

Goal/Plan/Loop/Session 可以继续保留自己的业务含义，但不要各自发明互不兼容的运行状态。

## 2. 建议 OpenSpec change

`unify-agent-run-state-machine`

必须先调研：

- `agent-execution-observability`
- `agent-lifecycle-management`
- `session-runtime-management`
- `session-recovery`
- `plan-execution-runtime`
- `loop-engineering-runtime`
- `goal-management`
- `operations` native context
- `multi-agent-group-chat`
- `permissions-approval`
- `agent-user-question`

如果现有 `operations` 已经能作为统一 substrate，优先扩展它，不新建第二套任务运行系统。

## 3. Canonical 状态模型

建议至少包含：

```text
CREATED
  -> PREPARING
  -> RUNNING
       -> WAITING_APPROVAL
       -> WAITING_USER
       -> PAUSED
       -> RETRYING
       -> BLOCKED
       -> STUCK
  -> VERIFYING
  -> COMPLETED

terminal:
FAILED
CANCELLED
```

最终枚举应由调研后决定，但必须定义：

- allowed transition；
- trigger；
- terminal/non-terminal；
- recoverability；
- timestamp；
- reason code。

## 4. Run 与现有对象关系

建议模型：

```text
Goal
 └── Run(s)
      ├── owner_type / owner_id
      ├── Session/Plan/Loop reference
      ├── Agent execution(s)
      ├── Tool calls
      ├── Artifacts
      ├── Verification
      └── child Runs
```

Multi-Agent 子任务用 parent/child run 表示时，仍需保留 Seat/turn routing 自身业务语义。

## 5. 功能需求

### R1. Canonical Run ID

所有长运行 Agent/Plan/Loop 执行必须能够关联稳定 `run_id`。

该 id 用于：

- logging；
- UI；
- cancellation；
- recovery；
- metrics；
- review；
- eval；
- future background runner。

### R2. Transition Guard

状态变化必须由 application/domain invariant 控制。

例如：

- COMPLETED 不能回 RUNNING；
- CANCELLED 不再接收新 tool execution；
- WAITING_APPROVAL 只有 approval/reject/cancel 才能离开；
- VERIFYING 完成后进入 terminal 或修复/重试路径；
- retry 次数受策略控制。

### R3. Run Event

定义统一事件：

```text
RunCreated
RunStarted
RunWaiting
RunResumed
RunRetrying
RunVerifying
RunCompleted
RunFailed
RunCancelled
RunStuck
```

事件只包含必要、安全元数据。

### R4. Recovery

应用重启后：

- terminal run 保持 terminal；
- 可恢复 run 按 owning runtime policy 恢复；
- 无法恢复的外部 CLI process 不伪装为仍 RUNNING；
- 必须变成明确的 interrupted/recovery outcome；
- 不能自动重放 destructive action。

### R5. Cancellation

统一 cancellation contract：

- UI cancel；
- parent cancel；
- timeout；
- shutdown。

子 run 的传播策略必须在 design 中定义。

### R6. Waiting semantics

区分：

- 等用户文字输入；
- 等权限审批；
- 等外部依赖；
- 暂停；
- stuck。

UI 不再用一个泛化 “waiting” 混合所有状态。

### R7. Verification phase

Coding task 可以进入 VERIFYING，用于：

- tests；
- review；
- security check；
- acceptance criteria。

与下一需求 Eval Platform 形成统一关联。

### R8. Compatibility

必须保留：

- 现有 Tauri command 名和序列化，除非 spec 明确批准 breaking change；
- 现有 session/plan/loop 数据；
- Web/mock parity。

数据迁移必须可回滚或有明确 forward-only migration policy。

## 6. UI 要求

先做最小统一状态展示组件：

- status badge；
- elapsed；
- waiting reason；
- retry；
- cancel/resume when allowed。

不要在本 change 提前实现完整 Mission Control。

## 7. 验收场景

1. API Agent 正常：CREATED→PREPARING→RUNNING→VERIFYING→COMPLETED。
2. 权限工具：RUNNING→WAITING_APPROVAL→RUNNING。
3. 用户问题：RUNNING→WAITING_USER→RUNNING。
4. Provider transient failure：RUNNING→RETRYING→RUNNING/FAILED。
5. 用户取消：任何允许状态→CANCELLED。
6. App 重启：不可恢复 CLI run 不保持假 RUNNING。
7. Parent run 取消时 child run 按策略终止。
8. 非法 transition 被 domain test 拒绝。

## 8. 测试要求

- State transition table unit tests；
- property/table-driven tests；
- persistence migration/recovery；
- cancellation race；
- duplicate terminal event idempotency；
- Web/mock parity；
- UI status component；
- Playwright waiting/approval/cancel；
- Desktop test 至少验证一次真实 Agent operation 状态事件。


## Codex + OpenSpec 强制执行规则

本需求不是让 Codex 直接开始改代码。执行时必须遵循以下顺序：

1. 先读取仓库根目录 `AGENTS.md`、`openspec/project.md`。
2. 检索 `openspec/specs/` 中与本需求相关的已确认主规范。
3. 检索 `openspec/changes/` 中未归档 change，确认是否已有重叠/冲突工作。
4. 如已有同能力 active change：
   - 优先扩展或完成现有 change；
   - 禁止创建重复的平行架构；
   - 如必须拆分，明确依赖关系和边界。
5. 如无现有 change，按仓库既有 OpenSpec 结构创建 proposal/design/tasks/spec delta。
6. 在改业务代码前执行：
   `openspec validate <change-name> --strict`
7. 按 `tasks.md` 顺序实施，不允许跳过验证任务。
8. 完成后执行所有适用的功能、自动化、UI、视觉、代码规范、性能与桌面运行时验证。
9. 所有验收通过后再归档：
   `openspec archive <change-name>`
10. 归档后执行：
   `powershell -ExecutionPolicy Bypass -File scripts/Update-OpenSpecArchiveIndex.ps1`
11. 一个需求完整结束、归档并提交后，才开始下一个需求。

如果当前 Codex 环境提供 `/opsx:new`、`/opsx:apply` 等 OpenSpec Skill，可使用它们创建/应用 change；但最终目录结构、验证与归档必须符合本仓库 `AGENTS.md`。

### 不允许做的事情

- 不允许为了通过测试而删除、降低或绕过既有测试/CI 门槛。
- 不允许使用 `git commit --no-verify` 或修改 hooks 来绕过规则。
- 不允许新增架构 allowlist 来掩盖新违规。
- 不允许 React 组件直接调用 Tauri `invoke()`。
- 不允许为本需求另建与现有 bounded context 平行的重复领域模型。
- 不允许在没有 OpenSpec proposal 的情况下直接实现架构/功能变更。
- 不允许顺手实现后续需求；发现后续依赖只记录，不提前扩大 scope。

## 全局 Definition of Done

任何涉及代码的 change，至少必须跑通仓库 `AGENTS.md` 当前要求的基础校验：

```bash
npm run lint:ci
npm run test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
openspec validate --specs --strict
openspec validate <change-name> --strict
```

按变更类型追加：

- 前端覆盖率：`npm run test:coverage`
- Contract：`npm run contracts:check`
- UI 行为：`npx playwright test`
- 桌面/Tauri/IPC：`npm run desktop:unit:test`、`npm run test:desktop`
- UI 视觉：对受影响页面至少覆盖 `futuristic` / `minimal` 两种样式、桌面宽度和窄宽度；优先使用现有 Playwright screenshot/visual 机制，如不存在则在本 change 中建立最小、稳定、可重复的视觉回归方案。
- 性能敏感路径：必须给出可重复的 benchmark/measurement evidence，CI 中优先使用确定性预算和结构性断言，避免共享 runner 上脆弱的固定毫秒断言。
- Windows/macOS/Linux 原生验证：只报告实际运行过的平台；统一使用 `PASSED` / `FAILED` / `BLOCKED` / `NOT RUN`。

### 完成报告必须包含

- OpenSpec change 名称
- 新增/修改的 capability
- 关键架构决策
- 主要文件
- 数据迁移（如有）
- 测试结果
- UI/视觉验证结果（如适用）
- 性能结果（如适用）
- Desktop Smoke 各平台状态（如适用）
- 已知限制和后续依赖

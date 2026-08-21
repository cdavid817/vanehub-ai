# P1 — Agent Mission Control

## 1. 目标

建立统一的 Agent 运行控制台，收敛当前 Goal / Plan / Loop / Session / Group Chat / operations 等多个页面中的“运行状态查看”体验。

Mission Control 不是替代这些业务页面，而是提供：

> “现在有哪些 Agent/Run 正在干什么？谁卡住了？谁在等我？成本/进度/验证结果如何？”

## 2. 建议 OpenSpec change

`add-agent-mission-control`

前置条件：

- Unified Run State Machine 已完成；
- Eval Platform 至少有稳定 metrics/read model；
- 不要在本需求内重新发明运行状态。

调研复用：

- `agent-execution-observability`
- `goal-management`
- `plan-management`
- `loop-management-ui`
- `multi-agent-group-chat`
- `notification-system`
- `main-layout-ui`
- `session-workspace-tabs`

建议新 capability：`agent-mission-control`

## 3. 首页总览

至少显示：

- Running；
- Waiting approval；
- Waiting user；
- Retrying；
- Stuck/Blocked；
- Failed；
- Completed recently。

每个 Run 卡片：

- agent；
- task/title；
- state；
- elapsed；
- workspace/worktree；
- current phase；
- token/cost（有可靠数据才显示）；
- needs attention；
- verification status。

## 4. 过滤和排序

支持：

- status；
- agent；
- project；
- local/remote runner（未来字段可先隐藏）；
- newest/oldest；
- needs attention first。

大列表必须分页/虚拟化或有合理 bounded read model，避免把完整日志一起拉到首页。

## 5. Run Detail

点击 Run 后显示 tabs：

- Overview；
- Plan/Tasks；
- Timeline；
- Tools；
- Files/Artifacts；
- Review；
- Tests/Verification；
- Context；
- Usage；
- Logs。

不存在的能力显示明确 unavailable，不做假数据。

## 6. Attention Inbox

特别突出：

- WAITING_APPROVAL；
- WAITING_USER；
- STUCK；
- FAILED；
- review requested。

支持从 Mission Control 直接进入 owning Session/Review/Approval 页面。

第一版不要在 dashboard 内复制完整编辑器/聊天体验。

## 7. Actions

根据 Run state 和权限提供：

- Open；
- Cancel；
- Resume；
- Retry；
- Approve/Reject（如 permission contract 支持）；
- Review Changes；
- Run Verification。

所有 action 必须调用统一 service，不在页面写 runtime-specific branch。

## 8. Read Model

建议使用专门 query/read model：

```text
MissionControlRunSummary
```

不要为了 dashboard 加载每个 Run 的整个 aggregate。

字段应由 owning contexts 投影，不让 frontend 拼接 6 个 N+1 请求。

## 9. 实时更新

优先复用现有 event/operation notification 机制。

要求：

- 高频 token event 不需要让 dashboard 每 token 重渲染；
- state transition 及时更新；
- terminal event 立即 flush；
- app 恢复焦点后能 reconcile。

## 10. UI/视觉

桌面：高信息密度，不做 hero dashboard。

建议：

```text
summary strip
attention queue
active runs table/cards
recent completed
```

窄宽：
- summary 可横向/换行；
- run detail 用 tab/dropdown；
- 不让 10 列 table 硬挤。

两种主题都验证。

## 11. 验收

1. 同时运行多个 Agent 时列表状态正确。
2. waiting approval 置顶并能跳到审批。
3. completed run 不继续显示 running timer。
4. retry/stuck 有 reason。
5. 关闭再打开应用后 read model 与持久化状态一致。
6. 100+ 历史 run 时首页仍 bounded。
7. Web/mock 提供 deterministic mock runs。
8. 点击 Review 进入同一 Code Review Center，而不是复制一套。

## 12. 测试

- read model aggregation；
- status/filter/sort；
- event coalescing；
- UI actions；
- Playwright 多 run + waiting + failed；
- Visual 四组合；
- Desktop smoke 验证真实 operation 能进入 Mission Control；
- 性能：大历史数据不能产生 N+1 native query。


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

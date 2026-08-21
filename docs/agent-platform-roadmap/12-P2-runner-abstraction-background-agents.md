# P2 — Runner Abstraction + Background Agents

## 1. 目标

把“Agent 运行位置”从默认本机进程抽象成 Runner：

```text
Agent
  -> Run
     -> Runner
        ├── Local
        ├── SSH
        ├── Docker/Sandbox
        └── Future Cloud
```

在统一 Runner 之上支持 Background Agent：

- UI 关闭当前页面后继续运行；
- 用户可从 Mission Control 恢复查看；
- 重启后有明确 recovery 状态；
- 受 permission/resource policy 治理。

## 2. 建议 OpenSpec change

`add-agent-runner-abstraction`

前置：

- Unified Run；
- Sandbox；
- Performance Governance。

调研：

- `remote-terminal-runtime`
- SSH workspace；
- session shell；
- project worktree；
- agent terminal runtime；
- desktop background lifecycle；
- operations；
- process gateway。

## 3. Runner Contract

建议：

```text
Runner
- capabilities()
- prepare()
- spawn()
- send_input()
- stream_events()
- cancel()
- inspect()
- cleanup()
- reconnect()/recover() where supported
```

Runner 不能包含 provider-specific prompt parsing；Provider 和 Runner 是两个维度：

```text
Provider = 怎么和 Codex/Claude/... 说话
Runner   = 在哪里/怎样运行它
```

## 4. Local Runner

把当前 native CLI process path 迁入 LocalRunner，保持行为兼容：

- cwd；
- env；
- PTY；
- cancellation；
- output；
- exit code；
- resume。

这是兼容重构，不应改变用户行为。

## 5. SSH Runner

复用已有 SSH connection pool/runtime。

要求：

- remote cwd；
- command availability；
- PTY/exec；
- keepalive；
- reconnect policy；
- network drop；
- cancellation；
- host key；
- credential boundary。

不要再建第二套 SSH transport。

## 6. Sandbox/Docker Runner

如果 Sandbox change 已有 process isolation，优先复用。

Docker 支持可以分 Stage 2：

- workspace mount；
- read/write scope；
- image allow/pin；
- resource limits；
- cleanup；
- no privileged mode by default；
- secret injection explicit。

## 7. Background Execution

Run 与页面生命周期解耦：

- 关闭 session tab 不 cancel；
- app minimize 不 cancel；
- app process exit 是否能继续，取决于 runner 类型和明确 spec：
  - Local child process 通常随 app policy；
  - SSH remote process 需要 reconnect strategy；
  - future daemon/cloud 可真正持续。

不要把“页面后台”宣传成“应用退出后仍然后台”除非确实实现。

## 8. Recovery

App restart：

- persisted Run；
- runner reference；
- inspect remote/local state；
- reconnect when supported；
- 否则标记 interrupted；
- destructive tool 不自动 replay。

## 9. UI

Run 创建时可选择：

- Local；
- SSH workspace；
- Sandbox/Docker（如果可用）。

Mission Control 显示 runner badge 和 host。

## 10. 验收

1. 当前本地 CLI 行为迁移到 LocalRunner 无回归。
2. 同一 provider 可以 Local 和 SSH 各运行一次。
3. 关闭 session 页面，Run 继续并可从 Mission Control 看到。
4. SSH 网络断开进入明确状态，可按 policy reconnect。
5. cancel 能终止 owning remote/local process。
6. app restart 后不能把已死 local process 显示为 running。
7. Runner error 与 Provider error 可区分。
8. secrets 只注入被批准的 runner/process。

## 11. 测试

- runner contract conformance；
- fake local process；
- fake SSH transport；
- cancellation race；
- disconnect/reconnect；
- cleanup；
- background UI navigation；
- Desktop integration；
- resource growth with concurrent runners。


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

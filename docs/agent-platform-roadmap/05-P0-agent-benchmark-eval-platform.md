# P0 — Agent Benchmark / Eval Platform

## 1. 目标

建立 VaneHub 自己的 Agent Evaluation Platform。

现有 Vitest/Playwright/Desktop Smoke 回答的是：

> VaneHub 软件有没有 Bug？

本需求需要回答：

> OnePiece / Codex / Claude Code / Gemini / OpenCode 在同一任务上谁完成得更好、成本多少、耗时多少、用了哪些工具、是否引入回归？

## 2. 建议 OpenSpec change

`add-agent-evaluation-platform`

调研并复用：

- `agent-context-quality-evaluation`
- `agent-execution-observability`
- `agent-context-measurement`
- `usage` 相关 spec/read models
- `loop-engineering-runtime`
- `plan-execution-runtime`
- `onepiece-subagents`
- `project-worktree-management`
- unified Run（应先完成）
- Context Engine evidence manifest（应先完成）

建议新 capability：

- `agent-evaluation`
- 如需 UI，增加 `agent-evaluation-ui` 或在同 capability 中明确 service/UI contract。

## 3. Benchmark Task 格式

定义版本化 Task Manifest，例如：

```yaml
id: fix-null-auth-token
version: 1
category: bugfix
fixture: ./fixtures/auth-bug
prompt: ...
timeout: 20m

acceptance:
  commands:
    - npm test
  expected_files: []
  forbidden_patterns: []

metrics:
  collect_context: true
  collect_tokens: true
  collect_tools: true
```

Manifest 不应允许任意未授权 host command。执行必须受 Runner/Sandbox policy 约束。

## 4. Benchmark 分类

至少支持：

### Coding
- bug fix；
- feature；
- refactor；
- tests；
- code review。

### Tool Use
- file read；
- search；
- LSP；
- terminal；
- patch。

### Context
- definition recall；
- reference recall；
- test recall；
- memory recall；
- budget efficiency。

### Planning
- decomposition；
- dependency；
- replanning。

### Multi-Agent
后续扩展：
- delegation；
- conflict resolution；
- verification。

MVP 不需要一次做全；先实现框架 + 3~5 个稳定 fixture task。

## 5. 执行隔离

每次 benchmark run：

- 创建独立 fixture copy/worktree；
- 固定初始 revision；
- 固定 task manifest version；
- 记录 Agent/provider/model/config snapshot；
- 禁止不同 Agent 共享上一次运行生成文件；
- timeout/cancel 后清理；
- secret 不进入结果 artifact。

## 6. 指标

### Outcome
- task success；
- acceptance checks pass；
- regression count；
- patch correctness；
- human intervention count。

### Efficiency
- input/output/cache tokens（可获得时）；
- estimated tokens（不可获得时明确标记）；
- wall time；
- tool calls；
- retries；
- replans；
- cost（只有有可靠 pricing snapshot 才计算）。

### Context
- evidence selected；
- relevant evidence recall；
- irrelevant evidence ratio；
- evidence token efficiency。

### Reliability
- failure category；
- timeout；
- stuck；
- recovery；
- flaky result。

## 7. Judge 设计

优先级：

1. deterministic tests；
2. static assertions；
3. repository diff rules；
4. structured rubric；
5. LLM-as-Judge 作为补充。

不能让 LLM-as-Judge 覆盖确定性测试失败。

LLM judge 必须记录：

- judge model；
- prompt/rubric version；
- temperature/seed（如支持）；
- evidence；
- score confidence/notes。

## 8. Agent Arena

同一 task 可以选择多个 Agent 顺序或并行运行。

结果表：

```text
Agent      Success  Tests  Tokens  Time  ToolCalls  Retries
OnePiece   PASS     42/42  ...     ...   ...        ...
Codex      PASS     42/42  ...     ...   ...        ...
...
```

排名算法必须透明、版本化，默认不把不同缺失指标硬凑成一个“神秘总分”。

## 9. UI

新增 Eval/Benchmark 页面：

- benchmark catalog；
- run configuration；
- running status；
- results table；
- compare runs；
- task detail；
- diff/verification；
- context/tool timeline；
- export JSON。

第一版 leaderboard 只做本地结果，不做公开云排行榜。

## 10. 结果持久化

SQLite 保存：

- task/version；
- run；
- agent snapshot；
- metrics；
- verification；
- artifact references。

大体积 logs/diff 不应无界塞数据库；遵循现有 retention/logging architecture。

## 11. 验收标准

1. 同一 fixture 能分别用至少 OnePiece + 一个 CLI Agent 跑。
2. 每个 run 使用干净隔离环境。
3. 结果可比较且知道 Agent/model/config。
4. deterministic acceptance test 优先决定 pass/fail。
5. token 不可获得时不会伪造精确数字。
6. failed/timeout/stuck 有分类。
7. results 可导出 JSON。
8. Context Engine 的 evidence metrics 可以挂到 eval run。
9. benchmark 本身失败与 Agent task 失败可区分。

## 12. 测试

- Task manifest schema；
- fixture reset；
- verifier；
- metrics aggregation；
- cost snapshot；
- result persistence；
- UI filtering/comparison；
- Playwright complete mock benchmark；
- Desktop test 一个最小真实 benchmark；
- benchmark framework 自身必须可在 CI 用 deterministic fake agent 跑，不依赖付费外部模型。


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

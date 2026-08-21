# P1 — Runtime Performance Budgets

## 1. 目标

扩展现有 `runtime-performance-governance`。

当前已有 native release profile、frontend chunk budget、blocking work off main thread、batched queries、chat stream coalescing 等基础。本需求增加**运行时可测量性能预算**，重点覆盖 Agent Engineering 热路径。

不要新建平行 performance subsystem。

## 2. 建议 OpenSpec change

`expand-runtime-performance-budgets`

主要扩展：

- `runtime-performance-governance`
- `agent-context-measurement`
- `agent-execution-observability`
- `remote-terminal-runtime`
- `lsp-code-intelligence`
- Context Engine / Run

## 3. 指标体系

### App
- cold start；
- time to interactive；
- idle memory；
- CPU idle；
- main-thread long task。

### Agent Runtime
- event propagation latency；
- token stream batching；
- run state transition overhead；
- cancellation latency；
- concurrent runs resource growth。

### Context Engine
- candidate collection；
- ranking；
- budgeting；
- evidence projection；
- index query；
- context bytes/tokens。

### Code Intelligence
- Tree-sitter incremental update；
- LSP definition/references；
- workspace indexing；
- search P50/P95。

### Persistence
- Mission Control list；
- session history；
- eval result query；
- terminal search；
- N+1 detection/structural guards。

## 4. Budget 分类

不要所有指标都使用共享 CI 固定毫秒。

分：

### Deterministic Gate
例如：
- query count；
- max loaded rows；
- buffer bounds；
- allocations/data-size bounds；
- max chunk size；
- no N+1；
- no synchronous blocking path。

### Dedicated Benchmark Evidence
在相对稳定 runner 或本地 benchmark：
- latency；
- throughput；
- memory。

### Informational Telemetry
不作为 PR hard gate：
- real-device cold start；
- model/network dependent latency。

## 5. Benchmark Harness

提供版本化 fixture：

- small repo；
- medium repo；
- large synthetic repo；
- 100 sessions；
- 1k runs；
- long terminal；
- high-rate token stream。

benchmark 结果包含：

- commit；
- platform；
- build profile；
- dataset version；
- metric；
- baseline；
- delta。

## 6. Regression Policy

设置预算时基于实际 baseline，不要凭文档示例硬写数字。

OpenSpec design 必须先跑 baseline，然后提出 threshold，例如：

```text
budget = baseline + justified headroom
```

对统计性 metric 可以使用相对回归阈值，而不是绝对极小毫秒。

## 7. Performance UI（可选）

开发者/Diagnostics 页面可以展示：

- last performance measurements；
- context timings；
- run event lag；
- memory estimate。

普通用户默认隐藏。

## 8. 验收

1. 有可重复 benchmark command。
2. 结果能区分 platform/build/dataset。
3. Context Engine 有 latency + occupancy evidence。
4. 100/1000 run 列表没有 N+1。
5. token stream 不 per-token rebuild whole UI state。
6. blocking workspace work 不回归主线程。
7. 超预算 gate 给出 metric + baseline + measured。
8. CI 不因共享 runner 抖动频繁 flaky。

## 9. 测试

- benchmark harness unit；
- dataset determinism；
- performance parser；
- budget compare；
- regression negative fixture；
- UI long list；
- Desktop performance evidence按平台独立记录。


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

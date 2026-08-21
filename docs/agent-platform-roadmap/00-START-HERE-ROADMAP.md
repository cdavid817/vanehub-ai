# VaneHub AI — Codex + OpenSpec 实施路线图

> 目标：把 VaneHub AI 从“统一管理多个 AI Coding CLI 的桌面工作台”进一步演进成可运行、协调、评测、审查和治理 Coding Agents 的 Agent Engineering Platform。
>
> 本路线图按 **依赖关系 + 风险 + 产品价值** 排序。Codex 必须逐个实施，禁止一次性并行铺开所有需求。

## 基线认知

开始前以仓库当前代码和 OpenSpec 为准，而不是以本文件对现状的描述为准。

当前仓库已经具有并必须复用的能力包括但不限于：

- React 19 + TypeScript + Tauri 2 + Rust + SQLite；
- Web/mock 与 Tauri runtime adapter 双边界；
- `agent_runtime`、`sessions`、`workspaces`、`tooling`、`operations` 等 native bounded contexts；
- Agent provider runtime；
- OnePiece native agent；
- Tree-sitter / LSP / retrieval / context compaction / context optimization / context quality evaluation；
- Multi-Agent Group Chat；
- Goal / Plan / Loop / Worktree / SSH / terminal；
- Skills / MCP / Hooks / permissions；
- CI、Vitest、Playwright、Desktop Smoke、coverage、contract checks；
- OpenSpec specs + active changes + archive governance。

不要复制这些能力。所有需求都应该先回答：

> “现有哪一个 capability 应该扩展？真正缺的 delta 是什么？”

## 执行优先级

| 顺序 | 优先级 | 需求 | 建议 change 名 | 主要依赖 |
|---|---|---|---|---|
| 01 | P0-Foundation | Architecture Fitness Functions | `enforce-architecture-fitness-functions` | 无 |
| 02 | P0 | Agent Code Review Center | `add-agent-code-review-center` | 01 |
| 03 | P0 | Unified Context Engine | `unify-agent-context-engine` | 01 |
| 04 | P0 | Unified Agent Run State Machine | `unify-agent-run-state-machine` | 01 |
| 05 | P0 | Agent Benchmark / Eval Platform | `add-agent-evaluation-platform` | 03、04 |
| 06 | P0 | Release Signing + Auto Update | `add-signed-release-auto-update` | 01 |
| 07 | P1 | Agent Mission Control | `add-agent-mission-control` | 04、05 |
| 08 | P1-Security | Sandbox / Permission / Supply-chain Security | 优先复用 active changes | 01 |
| 09 | P1 | Provider Plugin SDK | `extend-provider-runtime-plugin-sdk` | 08 |
| 10 | P1 | Runtime Performance Budgets | `expand-runtime-performance-budgets` | 03、04 |
| 11 | P2 | Hybrid Local Model Runtime | `add-hybrid-local-model-runtime` | 03、09、10 |
| 12 | P2 | Runner Abstraction + Background Agents | `add-agent-runner-abstraction` | 04、08、10 |
| 13 | P3 | Plugin / Agent Marketplace | `add-governed-agent-marketplace` | 08、09 |

## 为什么把 Architecture Fitness 放在第一个

它本身不是最亮眼的产品功能，但后面的需求都会改动跨层架构。先把规则机器化，可以降低 Codex/Claude Code 自动开发时产生架构漂移的概率。

它应该保护而不是替代现有规则，例如：

- UI 不能直接 import/use Tauri invoke；
- Web/Tauri adapters 必须保持 service contract 一致；
- Rust DDD dependency direction；
- 禁止新增 Zustand/Redux/MobX；
- 新生产 TS/TSX 文件不超过项目既有规则；
- bounded context 间只通过公开 API/contract/event 交互。

## Phase Gate

### Gate A — 基础治理完成
完成 01 后才能开始大规模架构 feature。

### Gate B — 核心 Agent 工程闭环
02 + 03 + 04 + 05 完成后，VaneHub 应形成：

`上下文选择 -> Agent 执行 -> 统一 Run -> Diff/Review -> 自动验证 -> Eval`

### Gate C — 产品化与安全
06 + 07 + 08 + 09 完成后，重点从“能运行”升级到“可发布、可观察、可扩展、可治理”。

### Gate D — 规模化运行
10 + 11 + 12 完成后，允许本地/远端/后台/混合模型运行，同时受性能与权限边界约束。

### Gate E — 生态
13 最后做。没有 Sandbox、签名/来源治理、Provider SDK 前，不应先做 Marketplace。

## 给 Codex 的启动方式

第一次执行只给 Codex 下达下面这条指令：

```text
读取 AGENTS.md、openspec/project.md，以及 docs/requirements/00-START-HERE-ROADMAP.md
（如果需求包没有拷贝到仓库，则读取我提供的对应 Markdown 文件）。

现在只执行优先级第 01 个需求：
01-P0-architecture-fitness-functions.md

要求：
1. 先分析现有 specs、active changes 和代码，不要直接实现。
2. 创建或复用 OpenSpec change。
3. strict validate proposal 后再改代码。
4. 完成该需求定义的所有测试和验收。
5. 归档 change 并更新 archive index。
6. 输出完成报告。
7. 不要开始第 02 个需求。
```

第 01 个完成后，再明确让 Codex读取第 02 个文档，以此类推。

## 推荐放入仓库的位置

建议把本需求包复制到：

```text
docs/requirements/agent-platform-roadmap/
├── 00-START-HERE-ROADMAP.md
├── 01-P0-architecture-fitness-functions.md
├── 02-P0-agent-code-review-center.md
...
└── 13-P3-plugin-agent-marketplace.md
```

这些文档是“实现输入”，真正的产品规范仍以实施过程中创建并最终归档进 `openspec/specs/` 的 OpenSpec capability 为唯一真源。


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

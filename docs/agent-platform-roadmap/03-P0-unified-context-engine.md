# P0 — Unified Context Engine

## 1. 目标

把当前已经存在的 Tree-sitter、LSP、retrieval/vector search、file reference、cross-session memory、context measurement、compaction、optimization、quality evaluation 等能力收敛到一个统一的 **Context Engine**。

本需求的重点不是“再做一个代码搜索”。

重点是：

> 在给定上下文预算下，根据当前任务自动选择、排序、裁剪、解释和验证最有价值的上下文证据。

## 2. 建议 OpenSpec change

`unify-agent-context-engine`

必须先阅读并复用至少：

- `agent-context-compaction`
- `agent-context-compaction-control`
- `agent-context-evidence-projection`
- `agent-context-measurement`
- `agent-context-optimization`
- `agent-context-quality-evaluation`
- `agent-cross-session-memory`
- `retrieval-vector-search`
- `lsp-code-intelligence`
- `lsp-server-management`
- `session-project-inspection`
- file reference 相关 spec
- OnePiece runtime specs

建议新增 capability：`agent-context-engine`，并通过 delta 修改相关现有 capability，而不是复制已有 compaction/optimizer。

## 3. 核心架构

```text
Task / User Turn
      │
      ▼
Context Request
      │
      ▼
Context Planner
      │
      ├── Explicit References
      ├── File/Text Retrieval
      ├── Tree-sitter Symbols
      ├── LSP Definition/References
      ├── Code Graph Evidence
      ├── Tests
      ├── Workspace State
      └── Memory
      │
      ▼
Candidate Normalizer
      │
      ▼
Context Ranker
      │
      ▼
Context Budgeter
      │
      ▼
Evidence Set
      │
      ▼
Provider Request Builder
```

Compaction/optimization 仍处理“已有会话上下文太大”的问题；Context Engine 处理“新一轮究竟主动注入哪些证据”的问题。两者必须有清晰边界。

## 4. 核心模型

建议统一内部模型：

```text
ContextCandidate
- id
- source_kind
- source_ref
- content_ref / lazy loader
- token_estimate
- relevance_score
- freshness
- authority
- redundancy_group
- required/protected flags
- safe fingerprint
- metadata

ContextBudget
- total
- reserved_system
- reserved_task
- reserved_recent_turns
- evidence_budget
- reserve

ContextEvidence
- candidate id
- selected range
- reason codes
- score breakdown
- occupancy
- provenance
```

避免让各来源直接向 provider request 追加自由文本。

## 5. 功能需求

### R1. Context Planner

输入至少包括：

- user turn/task；
- session/project/worktree；
- selected agent/model context capacity；
- explicit file references；
- current plan/task if any；
- current budget policy。

输出候选检索计划。

### R2. 多源候选收集

必须能够统一接收已有能力产出的候选：

- explicit file refs 优先；
- symbol definitions；
- references/call hierarchy（在 LSP 支持时）；
- text/vector retrieval；
- relevant tests；
- recent changes；
- memory；
- authoritative plan/task state。

来源失败时局部降级，不因为一个 LSP server 崩溃让整个 turn 失败。

### R3. Ranking

Ranking 第一版可以采用 deterministic weighted ranking + optional model rerank。

评分至少考虑：

- explicitness；
- semantic relevance；
- symbol relation；
- path proximity；
- recency；
- authority；
- duplication；
- estimated cost。

必须可解释：每个入选 evidence 有 reason codes。

### R4. Deduplication

同一代码片段可能由：

- text search；
- Tree-sitter；
- LSP；
- vector retrieval

重复命中。

必须 fingerprint + overlap merge，避免浪费 token。

### R5. Budgeting

支持版本化 Context Budget Policy。

示例：

```text
32K provider budget
- system: fixed/reserved
- recent conversation: reserved
- explicit user refs: protected
- code evidence: bounded
- memory: bounded
- plan/task state: bounded
- emergency reserve: bounded
```

不能只按字符硬截断代码导致语义边界破坏。

代码片段优先按 symbol / complete line range / complete tool result 边界裁剪。

### R6. Evidence Projection

模型输入中要有简洁的来源标识，例如：

- path；
- line range；
- symbol；
- evidence type。

不要把内部 score/debug metadata 全部塞给模型。

### R7. Context Inspector UI

在 Session/OnePiece 调试或高级视图中提供 Context Inspector：

- 本轮预算；
- 已选 evidence；
- source；
- line range；
- token estimate；
- 为什么选中；
- 为什么淘汰（可只保留 top rejected summary）；
- compaction 是否触发。

普通用户界面不应被大量 debug 信息淹没。

### R8. Measurement / Eval integration

Context Engine 产生的 evidence manifest 必须能够被后续 Eval Platform 使用：

- precision/recall；
- useful evidence；
- missing evidence；
- token efficiency。

### R9. Privacy / Logging

日志只能记录：

- source kind；
- counts；
- safe fingerprints；
- score buckets；
- token/character estimates；
- reason codes；
- latency。

禁止把源代码、prompt、memory 原文写入统一诊断日志。

## 6. 非目标

第一版不要求：

- 构建完整全语言 knowledge graph；
- 用 LLM 对所有候选做昂贵 rerank；
- 替换现有 compaction optimizer；
- 改写所有 provider adapter。

## 7. 验收标准

1. 明确 `@file` 引用不会被低分候选挤掉。
2. 用户问某函数 Bug，definition + tests + callers 能进入候选。
3. LSP 不可用时能退化到 Tree-sitter/retrieval。
4. 同一片段被 3 个来源命中时只占一份主要 token。
5. 超预算时按策略淘汰低价值证据，不切坏受保护内容。
6. 每次选择有可解释 reason codes。
7. Context Inspector 能查看本轮 evidence manifest。
8. Web/mock 和 Tauri 使用同一 service contract。
9. 现有 compaction/optimization contract 继续通过。

## 8. 测试与 Benchmark

建立最小 context benchmark dataset：

- definition retrieval；
- cross-file reference；
- test discovery；
- explicit ref preservation；
- duplicate elimination；
- LSP unavailable fallback；
- budget pressure；
- memory relevance。

测量：

- Recall@budget；
- Precision@budget；
- useful tokens / total evidence tokens；
- candidate collection latency；
- ranking latency；
- duplicate saving；
- provider budget overflow rate。


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

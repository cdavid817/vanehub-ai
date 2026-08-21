# P2 — Hybrid Local Model Runtime

## 1. 目标

让 OnePiece / API Agent 能使用本地或私有模型服务，并支持按任务类型做受控的 Hybrid Routing。

目标场景：

- Ollama；
- LM Studio；
- vLLM；
- SGLang；
- 通用 OpenAI-compatible local endpoint；
- 企业内网兼容服务。

不要把每个本地推理服务写成 Agent 特例，优先通过现有 provider/API runtime 抽象。

## 2. 建议 OpenSpec change

`add-hybrid-local-model-runtime`

前置：

- Unified Context Engine；
- Provider SDK；
- Runtime Performance Budgets。

调研：

- `api-agent-runtime`
- `agent-provider-runtime`
- `agent-lifecycle-management`
- `native-model-discovery`
- custom model/provider directory
- OnePiece profiles

## 3. Endpoint Profile

统一 profile 至少描述：

- provider/runtime kind；
- base URL；
- interface format；
- model id；
- context window（来源/用户覆盖要标识可信度）；
- capabilities；
- auth（可空，本地常无 key）；
- timeout；
- privacy classification。

不得仅凭模型名称猜 context window/capability。

## 4. Local Discovery

本地服务 discovery 应是显式/安全的。

支持：
- 用户手工 Base URL；
- 可选 localhost 常见 endpoint probe；
- timeout；
- 不扫描整个 LAN；
- 不自动上传任何代码。

发现只做 readiness/model metadata，不执行真实工作任务。

## 5. Hybrid Routing

第一版支持规则路由：

```text
task class -> preferred profile -> fallback profile
```

例如：

- summarization -> local；
- embeddings -> local；
- simple classification -> local；
- code review -> strong cloud；
- planning -> strong cloud。

路由规则必须：
- 用户可见；
- 可禁用；
- 有 fallback；
- 记录 reason；
- 不泄露被标记 `local-only` 的内容到 cloud fallback。

## 6. Privacy Policy

为任务/上下文定义数据策略：

- `cloud-allowed`
- `local-preferred`
- `local-only`

`local-only` 失败时：
- 不自动 fallback 到 cloud；
- 明确等待用户选择。

## 7. Capability Adaptation

本地模型常缺：
- tool calling；
- image；
- structured output；
- reasoning field。

Runtime 必须 capability negotiate，不假装支持。

## 8. Context Window

Context Engine 读取选中 profile 的预算。

如果 provider metadata 不可靠：
- 使用 configured conservative value；
- 标记 estimate；
- 防止超上下文无限 retry。

## 9. UI

Agent/Profile 设置：

- endpoint；
- model discovery；
- verify；
- capability；
- privacy；
- routing rules。

显示 Local 标签但不以“local=secure”做无依据保证。

## 10. 验收

1. 连接一个 OpenAI-compatible localhost model。
2. 能列模型/验证 endpoint（按服务支持能力）。
3. OnePiece 用本地 profile 完成纯文本 turn。
4. unsupported tool calling 能在执行前识别。
5. local-only 内容不 cloud fallback。
6. local provider down 时按 policy fallback 或等待用户。
7. Context Engine 使用正确预算。
8. usage 不可得时不伪造 token billing cost。

## 11. 测试

- fake local HTTP server；
- model list variations；
- timeout；
- malformed response；
- missing tools；
- privacy fallback；
- context limit；
- UI profile；
- Desktop localhost integration；
- performance：大 streaming response 不冻结 UI。


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

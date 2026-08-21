# P1 — Provider Plugin SDK

## 1. 目标

在现有 `agent-provider-runtime` 的静态 in-process provider registry 基础上，演进为**可扩展但受治理**的 Provider Plugin SDK。

目标：

> 新增 CLI Agent 不应该继续在 Session/runtime 中到处写 provider-id 分支。

但第一阶段仍要尊重现有 spec 的 deterministic static registration，只有当 OpenSpec 明确批准安全边界后才进入动态加载。

## 2. 建议 OpenSpec change

`extend-provider-runtime-plugin-sdk`

必须先读取：

- `agent-provider-runtime`
- `agent-lifecycle-management`
- CLI config/parameter specs
- `plugin-integration-management`
- `local-extension-management`
- Sandbox/Permission 完成结果

## 3. 分阶段实现

### Stage 1 — Internal SDK

先把 built-in provider contract 完整抽象成 SDK，仍然静态编译/注册。

Provider contract 至少覆盖：

- metadata；
- executable detection；
- readiness；
- capabilities；
- launch；
- prompt/input translation；
- output/event parsing；
- resume；
- cancellation；
- permission mapping；
- model/reasoning options；
- usage extraction；
- version detection；
- health diagnostics。

### Stage 2 — Manifest

定义版本化 manifest：

```yaml
schemaVersion: 1
id: example-cli
name: Example CLI
runtime: cli
executables:
  - example

capabilities:
  terminal: true
  resume: true
  images: false
  usage: true
```

Manifest 不能包含任意 executable script 作为无审查 install hook。

### Stage 3 — External Provider

只有 Sandbox / package provenance / permission model 已完成后，才允许外部 provider package。

动态扩展必须定义：

- SDK compatibility；
- manifest validation；
- signature/source；
- permission；
- lifecycle；
- update；
- disable/quarantine。

## 4. Provider-neutral runtime

以下模块禁止根据 provider id 选择业务路径：

- Session orchestration；
- generic Agent Run；
- usage page；
- Mission Control；
- Review；
- Eval。

Provider-specific 逻辑只存在 provider adapter 内。

## 5. Capability Negotiation

不能根据名字猜能力。

调用方必须：

```text
registry.resolve(id)
 -> provider.capabilities()
 -> request only supported feature
```

unsupported 返回结构化 error。

## 6. Parser Contract

CLI output parser 要支持：

- stdout/stderr chunks；
- structured JSON events（如 provider 支持）；
- text fallback；
- session/resume id；
- usage；
- tool event；
- completion/failure。

Parser 必须 robust to partial UTF-8/chunk boundaries。

## 7. Compatibility Test Kit

SDK 提供 provider conformance suite。

任意 provider 至少通过：

- deterministic registration；
- duplicate id reject；
- availability no side effect；
- launch arg mapping；
- cancellation；
- output parsing；
- resume；
- unsupported capability；
- sensitive args redaction；
- version detection failure。

## 8. Developer Experience

增加文档：

```text
docs/provider-sdk/
- contract
- manifest
- example provider
- conformance testing
- security rules
```

可以提供一个只用于测试的 `fixture-provider`，不要为了示例加入真正未支持 CLI。

## 9. 验收

1. 当前 5 个 built-in CLI 通过同一 conformance suite。
2. Session 层没有新增 provider identity branch。
3. 新增 fixture provider 不需要改 generic session orchestration。
4. duplicate provider id startup fail。
5. unsupported capability 清晰返回。
6. provider availability 不启动交互进程。
7. provider error 被映射为统一 classified error。
8. external provider（如本 change Stage 3 做）必须受 Sandbox/Trust 治理。

## 10. 测试

- contract/unit；
- parser fuzz/property tests（适合的部分）；
- partial chunk；
- bad manifest；
- duplicate id；
- permission；
- Web/Tauri service parity；
- 一个 fake CLI executable 的 desktop integration test。


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

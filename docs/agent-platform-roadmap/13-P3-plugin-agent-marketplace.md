# P3 — Governed Plugin / Agent Marketplace

## 1. 目标

建立一个受治理的 Plugin / Provider / Skill / Agent 分发体验。

这是最后阶段需求。

**没有完成 Sandbox / Permission / Supply-chain / Provider SDK 时禁止提前实现 Marketplace 安装执行。**

Marketplace 的核心不是“做一个卡片商店”，而是：

> 可验证来源 + 可理解权限 + 可控制更新 + 可回滚/禁用 + 兼容性检查。

## 2. 建议 OpenSpec change

`add-governed-agent-marketplace`

必须复用：

- `plugin-integration-management`
- `local-extension-management`
- remote skill registry active/main specs
- supply-chain governance
- sandboxed skill runtime
- Provider SDK
- permissions
- checksums/signature/provenance

如果 skill registry 已经覆盖 Marketplace 的大部分功能，本需求应只做统一 catalog/discovery UX 和 Agent/Provider package type 扩展。

## 3. Catalog Item

统一 catalog projection：

```text
id
name
type: skill | provider | plugin | agent-template
publisher
version
source
trust level
permissions
compatibility
checksum/signature/provenance
description
homepage
license
installed status
update status
```

## 4. Discovery

支持：

- search；
- category；
- type；
- verified/trust filter；
- installed；
- updates。

不要把下载量/评分做成安全可信度。

## 5. Install Review

安装前必须展示：

- publisher/source；
- exact version；
- requested permissions；
- executable/process access；
- filesystem；
- network；
- secrets；
- compatibility；
- verification result。

危险 permission 必须明确。

## 6. Install

安装必须：

1. fetch metadata；
2. verify checksum/signature/provenance；
3. verify compatibility；
4. unpack 到受控位置；
5. validate manifest；
6. register；
7. 不自动执行未审查 install script；
8. 写 audit record。

## 7. Update

更新前比较：

- version；
- manifest；
- permissions；
- trust/provenance；
- SDK compatibility。

如果 permission 扩大：

> 必须重新审批，禁止 silent update。

## 8. Disable / Remove / Quarantine

支持：

- disable；
- uninstall；
- quarantine on verification/runtime violation；
- 保留必要审计记录；
- 删除 credential grant 或要求用户确认保留。

## 9. Agent Templates

Marketplace 中的 “Agent” 第一版建议是**配置模板**而不是任意可执行二进制：

- system instruction；
- skill bindings；
- provider capability requirements；
- tool permissions；
- default model preference。

真正 executable provider 走 Provider Plugin SDK。

## 10. UI

Settings/Marketplace：

- catalog；
- detail；
- permissions；
- install/update；
- installed；
- verification badge；
- error/quarantine。

两主题、i18n、desktop/narrow。

## 11. 验收

1. verified catalog item 能安装。
2. checksum mismatch 被拒绝。
3. incompatible SDK 被拒绝。
4. 权限扩大 update 要重新确认。
5. untrusted item 有清晰风险提示。
6. disable 后 runtime 不再加载。
7. uninstall 不遗留 active executable registration。
8. quarantine item 不能运行。
9. marketplace UI 不把“popular”当“verified”。

## 12. 测试

- catalog schema；
- signature/checksum；
- malicious archive/path traversal；
- permission diff；
- compatibility；
- install transaction rollback；
- disable/remove；
- Playwright install/update flow；
- Desktop fixture registry；
- 网络失败/partial package；
- supply-chain negative tests。


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

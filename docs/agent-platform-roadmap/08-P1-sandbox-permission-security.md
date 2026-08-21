# P1-Security — Sandbox / Permission / Supply-chain Security

## 1. 目标

在允许更多 Skills、Plugins、Providers、Remote Runners 之前，完成统一的执行安全底座：

- filesystem 权限；
- process/command 权限；
- network 权限；
- secret 权限；
- resource limits；
- approval；
- signed/provenance metadata；
- remote registry supply-chain governance。

## 2. 特别说明：当前已有 active changes

本需求**禁止默认新建一个重复 change**。

Codex 必须先读取当前 `openspec/changes/`，尤其检查类似：

- `add-sandboxed-skill-tool-runtime`
- `add-remote-skill-registry-and-supply-chain-governance`
- `add-skill-configuration-management`
- skill evolution 相关 changes
- `publish-verifiable-checksums`

然后输出：

```text
本需求条目
-> 已在哪个 active change
-> 是否已完成
-> 是否缺 spec
-> 是否缺实现
-> 是否存在冲突
```

优先完成/扩展这些 active changes。

## 3. 统一 Permission Manifest

第三方能力应能声明：

```yaml
permissions:
  filesystem:
    read:
      - workspace/**
    write:
      - workspace/src/**
  network:
    domains:
      - api.github.com
  process:
    commands:
      - git
      - npm
  secrets:
    - github.token
```

Manifest 只是申请，不代表自动授权。

## 4. Trust Level

至少区分概念：

- Built-in / Trusted；
- Verified；
- Community；
- Local；
- Untrusted。

最终命名以现有 specs 为准。

Trust level 决定默认 policy，但用户显式审批仍受危险操作限制。

## 5. Filesystem Sandbox

要求：

- workspace-root bound；
- canonical path；
- symlink escape 防护；
- read/write 分离；
- hidden/system sensitive paths policy；
- temp directory ownership；
- output size limit。

## 6. Process Sandbox

要求：

- executable allowlist/capability；
- argv 结构化；
- shell 与 direct exec 区分；
- cwd 限制；
- env allowlist；
- timeout；
- process tree cancellation；
- stdout/stderr bound。

不允许把任意 Skill 输入直接拼进 shell string。

## 7. Network Sandbox

要求：

- default deny 或明确 policy；
- domain/host allow；
- scheme；
- redirects 重新验证；
- loopback/private network policy；
- proxy behavior；
- response size/time limit。

## 8. Secrets

Secrets 必须：

- 存现有 credential boundary；
- 按 capability 授权；
- 不注入不需要的 child process；
- 不显示在 logs；
- 不放入 Skill prompt；
- redaction before persistence。

## 9. Approval

危险动作使用统一 permission approval，而不是每种 Skill 自己做 dialog。

Approval 需要显示：

- 谁请求；
- 具体 action；
- target；
- scope；
- one-time/session/persistent（如果 policy 允许）。

## 10. Resource Limits

至少覆盖：

- wall time；
- child process count；
- output bytes；
- file bytes；
- network bytes；
- concurrent jobs。

CPU/memory hard isolation 如果跨平台实现成本高，可以在 design 中分阶段，但不能声称已有强隔离。

## 11. Supply-chain

远程 Skill/Plugin package：

- source；
- immutable version；
- checksum；
- signature/provenance（按现有 active change）；
- publisher；
- permissions；
- install-time review；
- update permission diff。

权限扩大时不得 silent auto-update。

## 12. 验收

1. Skill 尝试写 workspace 外被拒绝。
2. symlink 逃逸被拒绝。
3. 未声明 network host 被拒绝。
4. secret 不出现在 child env unless granted。
5. command timeout 杀死 process tree。
6. output 超限被安全截断/终止。
7. permission request 能在 UI 审批。
8. package checksum/signature mismatch 不能安装/更新。
9. 权限升级在 update 时被明确展示。
10. 日志不包含 secret/tool raw sensitive content。

## 13. 测试

安全需求必须包含 negative/adversarial tests：

- `../` traversal；
- symlink；
- command injection；
- redirect to disallowed host；
- secret exfiltration；
- huge output；
- hung process；
- corrupted package；
- replay/duplicate approval。

Desktop tests 要分别确认实际 OS 行为，不能把一个平台结果外推到另外两个。


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

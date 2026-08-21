# P0 — Agent Code Review Center

## 1. 产品目标

建立一个统一的代码审查闭环，使 VaneHub 内任意 Coding Agent 修改工作区后，用户可以在桌面端直接：

- 查看 changed files；
- 查看 unified / split diff；
- 按 hunk / line 评论；
- Accept / Revert hunk；
- 汇总 review comments；
- 把结构化反馈发回原 Agent；
- 运行 Review Agent / Tests / Security Checks；
- 最终 Commit / Create PR（PR 能力可作为后续可选 delta，不要为本 MVP 强行扩 scope）。

核心目标：

> Agent 写代码不等于任务完成；必须经过可观察、可评论、可验证的 Review 闭环。

## 2. 建议 OpenSpec change

`add-agent-code-review-center`

调研并优先复用：

- `project-worktree-management`
- `session-project-inspection`
- `agent-execution-observability`
- `agent-task-list`
- `permissions-approval`
- `session-workspace-tabs`
- `unified-log-management`
- existing Git diff commands / workspace bounded context

建议新 capability：`agent-code-review`

## 3. 核心数据模型

建议领域概念：

```text
ReviewSession
├── id
├── session_id
├── workspace_id / worktree_id
├── base_revision
├── head_revision / working-tree fingerprint
├── status
├── created_at
└── updated_at

ReviewFile
├── path
├── change_type
├── old_hash
└── new_hash

ReviewComment
├── id
├── file_path
├── side
├── line / hunk anchor
├── body
├── status
└── fingerprint

ReviewDecision
├── accepted
├── changes_requested
└── pending
```

评论锚点不能只依赖“绝对行号”。工作区继续变化时，需要使用 diff/hunk fingerprint 检测 stale comment。

## 4. 功能需求

### R1. Review 入口

当一个 Agent turn / Plan task / Loop phase 产生文件修改时：

- 页面提供 `Review Changes`；
- 可以从 Session Workspace 打开；
- 后续 Mission Control 可以复用同一个 review service。

### R2. Diff 获取

Diff 必须通过 workspaces/native service 获取。

支持：

- tracked modified；
- added；
- deleted；
- renamed（如果 Git 层可可靠提供）；
- untracked text files；
- binary files 只显示元信息，不尝试文本 diff。

必须限制：

- 单文件 diff 大小；
- 总 diff 大小；
- 超限提示和按文件加载；
- UTF-8 安全。

### R3. Review UI

至少提供：

- changed file list；
- file summary；
- unified diff；
- line number；
- added/removed context；
- inline comment；
- next/previous changed file；
- stale state 提示；
- loading/error/empty state。

UI 必须遵守项目 `futuristic` 和 `minimal` 视觉 token，不引入新的 UI framework。

### R4. Hunk 操作

MVP 支持：

- Copy diff；
- Revert whole file；
- Revert hunk；
- Accept hunk（记录 review decision，不等于改 Git index）。

任何 destructive revert 必须有权限/确认边界，并在底层验证 worktree fingerprint，避免覆盖用户新修改。

### R5. Send Feedback to Agent

用户可以选择多个 comment，形成结构化 review feedback：

```text
Review feedback:
1. src/a.ts:45-52 ...
2. src/b.rs:110 ...
```

必须：

- 保留 file / line / hunk 元数据；
- 在发送前检查 anchor 是否 stale；
- 通过 session/agent service 发送；
- 不直接在 UI 拼 provider-specific payload。

### R6. Automated Review

提供可插拔 Review Action：

- Run Review Agent
- Run Tests
- Run Security Checks

第一版不要强行引入新的多 Agent orchestration；优先复用现有 Agent/Plan/Loop/Tool runtime。

自动 review 结果也必须投影为 review findings，而不是只输出一大段聊天文本。

### R7. Persistence / Recovery

Review session 和 comments 至少在当前工作中可恢复。

如果项目现有 session persistence 适合持久化，则使用 SQLite；如果 OpenSpec 调研认定 MVP 可以 ephemeral，必须在 design 中说明为什么、何时丢失、后续迁移策略。

### R8. Observability

记录：

- review created；
- diff loaded；
- comment added/resolved；
- stale anchor；
- revert；
- feedback sent；
- automated checks result。

禁止日志记录代码全文、secret 或完整 diff 内容。

## 5. 安全要求

- 路径必须限制在 owning workspace/worktree。
- 拒绝 path traversal。
- Revert 前验证文件没有在 diff snapshot 后被未知修改。
- binary/huge file 不做不受控内存加载。
- Review feedback 中的代码内容仍视为 workspace data，不写入诊断日志。

## 6. UI/UX 验收

桌面宽度：
- 左侧 changed file list；
- 主区 diff；
- 评论操作不遮挡代码；
- 大 diff 滚动稳定。

窄宽度：
- file list 可折叠/切换；
- diff 不出现不可恢复的横向溢出；
- comment editor 可用。

两种视觉主题都必须检查。

## 7. 验收场景

1. Agent 修改 3 个文本文件，Review Center 正确列出。
2. 用户对某 added line 写评论并发送给原 Agent。
3. Agent 再修改同文件后，旧 anchor 能被识别为 stale 或可靠重定位。
4. 用户 revert 一个 hunk，不影响同文件其他不相关修改。
5. 工作区在 snapshot 后被外部修改时，危险 revert 被拒绝。
6. 大 diff 不冻结 UI。
7. Web/mock 模式能够模拟 review contract，不假装执行真实 Git 修改。
8. Tauri 模式真实读取 worktree diff。

## 8. 测试要求

- Rust：Git diff parsing / path safety / fingerprint / revert application。
- TS：Review service contract。
- Vitest：review state、comment stale logic、UI components。
- Playwright：从 Session 打开 Review、加评论、发送 feedback、revert mock hunk。
- Visual：两主题 + desktop/narrow。
- Desktop E2E：在测试仓库制造真实 diff，验证至少打开 review 和读取 changed files。
- 性能：大文件/大 diff 使用 bounded loading，不出现 O(N²) 重建。


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

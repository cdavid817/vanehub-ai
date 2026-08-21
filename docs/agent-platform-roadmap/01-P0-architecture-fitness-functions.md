# P0-Foundation — Architecture Fitness Functions

## 1. 目标

把 VaneHub AI 已经写在 `AGENTS.md`、`openspec/project.md`、ADR 和 service/runtime boundary 中的关键架构约束，尽可能转换成**可执行、可定位、CI 强制**的 Architecture Fitness Functions。

目标不是新增一种架构，而是防止未来 Codex、Claude Code、人工开发在迭代中把现有架构慢慢破坏。

## 2. 建议 OpenSpec change

`enforce-architecture-fitness-functions`

优先扩展 capability：

- `repository-governance`
- `frontend-runtime-architecture`
- `native-runtime-architecture`
- `continuous-integration`

如果现有 specs 已覆盖某条规则，只增加“自动强制层”的 delta，不重复声明业务规范。

## 3. 必须先调研

Codex 必须先盘点：

- `AGENTS.md`
- `openspec/project.md`
- `eslint.config.js`
- `scripts/`
- `src/contracts/contract-conformance.test.ts`
- Rust architecture tests
- `.github/workflows/ci.yml`
- `.husky/`
- `.claude/settings.json`
- 当前已有技术债豁免和其原因

输出一张：

`规则 -> 当前文档 -> 当前机器检查 -> 缺口 -> 本 change 是否补齐`

的矩阵。

## 4. 功能需求

### R1. Frontend runtime boundary 检查

自动拒绝：

- React page/component 直接 import `@tauri-apps/api`；
- component/page 直接调用 `invoke()`；
- page 内出现 runtime-specific native branch 来绕过 service adapter；
- 新能力只改 Tauri adapter、不维护 Web/mock adapter contract，或反之。

应优先使用 AST/ESLint/现有 contract test，而不是脆弱的纯文本 grep。

### R2. 状态管理约束

项目约束为 React 内置 state/context。

机器检查必须阻止新生产代码引入或使用：

- Redux
- Zustand
- MobX

如果 package 中存在历史依赖但生产代码未使用，需要在 change 中决定：
- 安全移除；或
- 明确记录临时技术债与删除任务。

禁止仅靠永久 allowlist 忽略。

### R3. Rust DDD dependency direction

补强现有 Rust architecture test，至少检查：

- `domain` 不依赖 Tauri/Rusqlite/filesystem/process/network/platform concrete adapters；
- `application` 不依赖 Tauri command/state、Rusqlite concrete connection；
- command 不执行 SQL，不直接构造外部进程；
- context A 不导入 context B 的 private repository/infrastructure/aggregate；
- 跨 context 只能使用公开 `api` / explicit contract / event；
- `bootstrap` 是 concrete dependency assembly 的唯一位置。

错误必须给出文件路径和违规依赖。

### R4. 文件/模块约束

复用现有 lint 规则并保证：

- 新 TS/TSX 生产文件遵守既有 300 行硬规则；
- 禁止为了本 change 新增 ESLint 技术债豁免；
- Rust `unwrap()/expect()` 生产路径遵守项目现有规则；
- TS 禁止 `any`、`@ts-ignore`。

### R5. Fitness 命令

提供一个开发者容易理解的统一入口，例如：

```bash
npm run architecture:check
```

该命令可以编排已有 lint/contracts/native architecture checks，但不要重复实现同一检查逻辑。

### R6. CI Gate

在 CI 中把 architecture fitness 作为明确 gate。

失败信息必须指出：

- rule id；
- 文件；
- 行/模块；
- 修复方向。

## 5. 设计要求

规则应分层：

```text
Architecture Rule Registry
├── Frontend rules
│   ├── service boundary
│   ├── runtime adapter parity
│   └── state management
├── Native rules
│   ├── DDD dependency direction
│   ├── context ownership
│   └── command thinness
└── Repository rules
    ├── file constraints
    └── dependency bans
```

不要创建一个几千行的单脚本。

## 6. 验收标准

- 人为添加一个 component 直接 `invoke()`，检查必须失败。
- 人为在生产代码 import Zustand，检查必须失败。
- 人为让 Rust domain import infrastructure，检查必须失败。
- 修复后检查恢复通过。
- CI 执行该 gate。
- 错误包含可定位文件信息。
- 没有新增 blanket allowlist。
- 现有 Web/Tauri contract test 不被削弱。

## 7. 测试要求

必须给每条新增 fitness rule 写正反例测试。

需要覆盖：

- frontend architecture fixtures；
- native architecture fixtures；
- script/unit tests；
- CI contract test。

本需求不应造成 UI 变化，因此无需新增 UI 截图；但必须跑基础全量验证。

## 8. 完成后给下一个需求提供的能力

后续所有 change 可以把本需求提供的 `architecture:check` 纳入实施前后验证，降低大规模重构的架构漂移风险。


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

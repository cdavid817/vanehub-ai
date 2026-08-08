# 开发环境搭建

> **两条工具链**：Node.js 负责前端与脚本，Rust 负责原生侧。两者都要装齐才能跑通完整校验。

## 前置要求

| 工具 | 版本 | 依据 |
|---|---|---|
| Node.js | **22** | CI 全部 job 使用 `node-version: 22`（`.github/workflows/ci.yml:28` 等四处） |
| npm | 随 Node 分发 | **必须用 npm**，仓库有 `package-lock.json`，不要切 pnpm / yarn |
| Rust | stable | CI 用 `dtolnay/rust-toolchain@stable`；仓库无 `rust-toolchain.toml`，不锁定具体版本 |

**平台**：Windows / macOS / Linux 均可开发。CI 的原生检查矩阵覆盖 `windows-latest` 与 `macos-latest`（`ci.yml:286-289`）。

> `package.json` 未声明 `engines` 字段，Node 22 这一要求来自 CI 配置而非包元数据。

## 安装

```bash
npm ci
```

Rust 依赖在首次 `cargo` 命令时自动拉取。**`rusqlite` 使用 `bundled` feature，不需要系统预装 SQLite。**

## 常用命令

### 开发

```bash
npm run dev          # 仅前端(Web/mock 模式,浏览器打开)
npm run tauri:dev    # 完整桌面应用
```

**`npm run dev` 起的是 Web/mock 模式**——没有原生能力，但界面与交互逻辑完整，适合调 UI。

### 构建

```bash
npm run build        # tsc + vite build + 前端分包检查
npm run tauri:build  # 桌面应用打包
```

**平台专用打包**：

| 脚本 | 目标 |
|---|---|
| `package:windows:x64` / `package:windows:arm64` | Windows |
| `package:macos:x64` / `package:macos:arm64` | macOS |
| `package:linux:x64` / `package:linux:arm64` | Linux |

## 校验命令

**改完必须全部跑通**（来自 `AGENTS.md`，逐字照抄参数）：

```bash
npm run lint:ci
npm run test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
openspec validate --specs --strict
```

**参数不能简化。**`AGENTS.md` 特别点名了三种本地能过、CI 会拦的写法：

| 错误写法 | 正确写法 | 差别 |
|---|---|---|
| `npm run lint` | `npm run lint:ci` | 后者带 `--max-warnings=0` |
| `cargo clippy`（不带参数） | 必须带 `--all-targets -- -D warnings` | 前者不检查测试代码，且警告不算失败 |
| 漏掉 `cargo fmt` | 必须跑，且带 `-- --check` | 不加 `--check` 会直接改文件而非报错 |

### 按改动类型追加

| 改了什么 | 还要跑 |
|---|---|
| 任何代码（CI 用它取代 `npm run test`） | `npm run test:coverage` |
| 覆盖率策略脚本 | `npm run coverage:policy:test` |
| 原生代码覆盖率 | `npm run coverage:native` + `coverage:check:native` |
| 跨边界类型 | `npm run contracts:check` |
| **文档** | `npm run docs:check`（含链接校验，覆盖整个 `docs/`） |
| UI 行为 | `npx playwright test` |
| 起了 proposal | `openspec validate <change-name> --strict` |

## 覆盖率门槛

**CI 用 `npm run test:coverage` 取代 `npm run test`，并强制门槛**（`coverage-policy.json`）：

| 范围 | 最低行覆盖率 |
|---|---|
| 前端整体 | **45.2%** |
| 原生整体 | **67%** |

**原生侧另有三个关键组，各要求 80%**：

| 组 | 覆盖的文件 |
|---|---|
| `agent-startup-and-terminal-control` | `agent_runtime/application/terminal_service.rs` |
| `mcp-routing` | `tooling/mcp/infrastructure/relay.rs` |
| `sqlite-transactions` | `sessions/infrastructure/transactions.rs`、`platform/database/mod.rs`、`platform/database/migrations.rs` |

**这三组的选择反映了风险判断**：终端控制、MCP 路由、SQLite 事务与迁移——出错代价最高、且最难靠人工测试覆盖的三块。

**改动这些文件时要特别注意补测试**，否则即使整体覆盖率达标，关键组门槛也会失败。

## E2E 测试

**配置在 `playwright.config.ts`**：

| 项 | 值 |
|---|---|
| 测试目录 | `./tests/e2e` |
| 默认端口 | `5174`（可用 `PLAYWRIGHT_PORT` 覆盖） |
| baseURL | 可用 `PLAYWRIGHT_BASE_URL` 覆盖 |
| 浏览器 | Chromium |
| webServer 命令 | `npm run dev -- --port <port> --strictPort` |

### 一个容易踩的坑：reuseExistingServer

**`reuseExistingServer: true`**（`playwright.config.ts:24`）意味着：**如果目标端口上已经有服务在跑，Playwright 直接用它，不会另起一个。**

**在多 worktree 并行开发时这会出问题**：另一个 worktree 的 `npm run dev` 占着 5174，你在当前 worktree 跑 `npx playwright test`，测的是**另一份代码**，而且不会有任何提示。

**规避**：确认 5174 上跑的是自己的服务，或用 `PLAYWRIGHT_PORT` 指定一个专属端口。

## 已知环境陷阱

### 迁移版本号冲突

**这是本项目最容易踩且最难诊断的问题。**

**症状**：应用启动或运行时报 `no such table: X`，但代码里明明有建表的迁移。

**原因**：`apply_migration` 是版本门控的——如果 `schema_migrations` 里已经有该版本号，迁移**被静默跳过**，它本该创建的表就是缺失的。代码注释记录了一次真实发生的情况（`src-tauri/src/platform/database/migrations.rs:267-270`）：

> `45-48, not 43-46`：`retrieval-vector-index` 与 `permissions-core` 在本分支开着的时候以 43、44 进入了 main，所以这四条要往后挪。**`apply_migration` 是版本门控的——第二个占用同一号码的迁移永远不会运行，它本该创建的表在启动时直接缺失。**

**并行开发时更容易触发**：同一台机器上的多个 worktree / 分支**共用同一个数据库文件**（路径由 `database_path(data_dir)` 解析，`platform/database/mod.rs:85`，data dir 对应应用标识符 `ai.vanehub.app`）。A 分支先用了版本 45，B 分支也用 45，则 B 的迁移在这台机器上永远不会执行。

**诊断方法**：直接查 `schema_migrations` 表，看该版本号对应的 `name` 是不是自己那条。**不要先假设是代码回归。**

**两种规避手段**：

1. 新增迁移前确认 `main` 上已用到的最大版本号（当前为 **48**），并留意其他在途分支
2. **用 `VANEHUB_APP_DATA_DIR` 让不同 worktree 指向不同数据目录**（`bootstrap/runtime.rs:345-350`，必须是绝对路径）

### 文档链接校验覆盖整个 docs/

`scripts/validate-docs.mjs` 的 `markdownRoots`（`:6-11`）包含 `README.md`、`README.zh-CN.md`、`README.ja.md` 与**整个 `docs/` 目录**。

**新增文档里的相对链接指向不存在的文件会直接让 CI 的 documentation job 失败**。跨阶段撰写文档时，前向引用要么最后补齐，要么先建占位文件。

### 文档构建必须是只读的

CI 在 `npm run docs:build` 之后跑 `git diff --exit-code`（`ci.yml:165-166`）。**如果文档构建会修改仓库内工作树文件，CI 会失败。**

### 300 行硬规则

`max-lines` 按**物理行**计（`eslint.config.js:55`，`skipBlankLines: false, skipComments: false`）。

| 情况 | 处理 |
|---|---|
| 测试文件 | 豁免（`eslint.config.js:59-61`） |
| 存量超限文件 | 在豁免清单中（`:67-78`），**禁止新增** |
| 新代码 | 一律 ≤300 行 |

写新代码时如果接近 300 行，先想拆分，不要指望加进豁免清单。

### 两个 binary target

`src-tauri/Cargo.toml` 声明了 `default-run = "vanehub-ai"`（`:7-10`）。**不要删掉它**——注释说明了原因：存在第二个 binary（权限钩子）后，Tauri 的 `tauri dev` / `tauri build` 内部调用不带 `--bin` 的 `cargo run`，会直接失败并报 "could not determine which binary to run"。

## Git worktree 开发

仓库 `.gitignore` 已忽略 `.claude/worktrees/`，可以在其下创建隔离工作区。

**注意事项**：

| 事项 | 说明 |
|---|---|
| **数据库是共享的** | 见上文迁移版本号冲突；用 `VANEHUB_APP_DATA_DIR` 隔离 |
| **dev server 端口会被复用** | Playwright 的 `reuseExistingServer`；用 `PLAYWRIGHT_PORT` 隔离 |
| **依赖需各自安装** | 每个 worktree 要单独 `npm ci` |
| **保持 npm** | 不要在 worktree 里用 pnpm，其目录布局与前端分包检查不兼容 |

## 提交规范

**提交信息一律英文**，遵循 Conventional Commits。允许的 type 有 12 种（`commitlint.config.mjs`）：

`build`、`chore`、`ci`、`deps`、`docs`、`feat`、`fix`、`perf`、`refactor`、`revert`、`style`、`test`

**依赖升级用 `deps`**：`deps(npm)` / `deps(cargo)` / `deps(actions)`——这是本仓库自加的 type，配置注释里写明了约定。

**PR 标题与描述同样一律英文。**

**不得绕过校验**：禁止 `git commit --no-verify`、禁止 `git push --force`，禁止为让校验通过而修改 `.husky/`、`.claude/settings.json`、eslint 豁免清单等。详见 [五层约束体系](constraints.md#禁止绕过)。

## 本地个性化配置

| 内容 | 写在哪 | 是否 gitignore |
|---|---|---|
| 权限放宽、本地实验 | `.claude/settings.local.json` | 是 |
| 个人化临时指令 | `CLAUDE.local.md` | 需确认已加入 |

**不要改仓库级 `.claude/settings.json`。**

## 相关文档

- [五层约束体系](constraints.md) —— 各层校验的触发时机
- [OpenSpec 工作流](openspec-workflow.md) —— 起提案与归档
- [数据层](../02-architecture/data-layer.md) —— 迁移机制细节
- [技术栈](../02-architecture/tech-stack.md) —— 各依赖的版本与理由

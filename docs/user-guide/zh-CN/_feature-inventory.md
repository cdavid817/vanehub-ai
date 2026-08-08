# 中文用户指南 · 功能清单（阶段一交付物）

> 本文件是撰写正式用户文档前的调研产物，用于确认写作范围。**确认后即可删除或归档，不进入 mdBook 目录**（mdBook 只读取 `src/`，本文件位于书根，不会被收录）。

## 调研范围

| 来源 | 覆盖内容 |
|---|---|
| `AGENTS.md` / `CLAUDE.md` | 项目约束、架构规则、校验命令 |
| `openspec/specs/` | 88 个已确认能力规范 |
| `openspec/changes/archive/archive-index.json` | 116 条已归档变更（2026-07-13 → 08-07） |
| `src-tauri/src/contexts/` | 11 个限界上下文的领域模型 |
| `src-tauri/src/commands/` | 237 处 Tauri command 声明 |
| `src/settings/settings-pages.ts` | 17 个设置页注册表 |
| `src/i18n/locales/zh-CN.json` | 2197 条界面文案（用于核实菜单/按钮名） |

**所有界面文案均取自 `zh-CN.json`，非推测。**

---

## 一、必须先决策的四处冲突

### 冲突 1：`docs/user-guide/zh-CN/` 已存在，且与英文版严格对等

现状：

```text
docs/user-guide/
├── screenshots.json          # 截图清单（受 CI 校验）
├── assets/screenshots/       # 截图资产
├── en/{book.toml, src/*}     # 8 个文件，181 行
└── zh-CN/{book.toml, src/*}  # 8 个文件，181 行 —— 与 en 逐文件行数完全一致
```

已有中文章节：`index` / `getting-started` / `first-session` / `skill-management` / `multi-agent-workflow` / `runtime-labels` / `troubleshooting`。

**两个问题：**

1. **任务书的目录结构（`01-快速开始.md` 等直接放在 `zh-CN/` 下）不会被 mdBook 收录**——mdBook 只读 `src/` 并以 `SUMMARY.md` 为准。放在书根等于文件存在但站点里看不到。
2. **规范要求双语对等。**`openspec/specs/user-guide-documentation/spec.md:6-12` 明确要求英文与简体中文提供**等价的导航、命令、运行时适用性、前置条件、结果与故障排查覆盖**。只写中文版会直接违反该规范，而 CI 的 `openspec` job 会校验主规范。

### 冲突 2：占位截图会让 CI 失败

`scripts/validate-docs.mjs` 有两道与截图相关的校验：

| 校验 | 要求 |
|---|---|
| `validateMarkdown` | 每个图片链接的**目标文件必须存在**，且 alt 文本非空 |
| `validateScreenshotInventory`（`:93-110`） | `screenshots.json` 每条目的 `path` 资产**必须存在**；`runtime` 只能是 `web-mock` 或 `desktop-reviewed` |

**因此，只要在文档里写出指向尚不存在的图片文件的 Markdown 图片链接（例如指向 `./images/04-01-session-list.png`），`npm run docs:check` 就会失败**，进而让 CI 的 documentation job 失败。

> 这一点在撰写本清单时被实证了两次：文中原本按任务书的写法给出示例，校验立刻报 `missing relative target`；改成行内代码后**仍然失败**——因为 `validate-docs.mjs` 是对原始文本做正则匹配，**不解析 Markdown**，反引号包裹并不能让它忽略。最终只能改成不含图片链接语法的纯文字描述。
>
> 这条额外说明了：**该校验无法通过"注释掉"或"代码块包裹"绕过**，占位截图方案必须从根本上改变。

此外规范要求截图必须由**具名 Playwright 场景**确定性生成（`spec.md:31-37`），当前只有 2 个场景（`create-session-en` / `create-session-zh-CN`），由 `npm run docs:screenshots:check` 验证。

### 冲突 3：任务书中三个主题在 `main` 上不存在

| 任务书条目 | 实际情况 | 依据 |
|---|---|---|
| `02-多Agent编排.md` — **DAG 工作流、拓扑可行性** | **不存在**。依赖图协调运行时已被**移除** | 迁移 45 `remove-multi-agent-coordination` 执行 `DROP TABLE coordination_runs`；`src/services`、`src/contracts`、`src-tauri/src/contexts` 中已无任何 coordination 引用。`agent_runtime/domain/workflow.rs` 只有单 Agent 的 `AgentLifecycle`/`AgentReadiness`，无依赖图或拓扑排序 |
| `07-检查点与会话迁移.md` — **检查点（Checkpoint）** | **不存在该用户功能**。代码中唯一的 "checkpoint" 是 Telegram 连接器的轮询游标（`communications/infrastructure/transports/telegram.rs`），属实现细节 | 全仓 `checkpoint` 检索仅命中 `communications` 上下文 |
| `07-检查点与会话迁移.md` — **Context Handoff / 会话迁移** | **不在 `main` 上**。仅存在于未合并分支 `feature/cross-cli-session-portability`；`openspec/specs/` 与归档中均无对应能力 | 分支列表 + specs/archive 检索 |

**连带影响**：建议使用案例中的第 2 条（DAG 流水线）与第 4 条（Context Handoff 迁移）无法编写。

**实际存在的替代品**：多 Agent 协作现在是**群聊席位 + `@` 交接**（`multi-agent-group-chat`），以及**Loop 工程化运行时**（目标驱动自动循环）。这两个都是真实且可写的。

### 冲突 4：`docs/zh/` 已有一套中文文档

上一轮工作已交付 `docs/zh/`（28 篇、6295 行，已接入文档站），定位是**功能介绍 + 架构实现细节**，双受众。

本任务的用户指南定位是**纯终端用户、任务导向、含截图与案例**——**定位不重叠**，但需要在两处导航之间建立明确指向，避免读者困惑。

---

## 二、功能清单

**状态口径**：

- **已实现** —— 有用户可见入口 + 已归档变更 + 代码实现
- **部分实现** —— 能力存在但覆盖面受限（已注明限制）
- **仅有 spec 未实现** —— 不写入正式文档

### 2.1 会话与执行

| 功能名 | 所属模块 | 入口位置 | 实现状态 |
|---|---|---|---|
| 创建会话 | sessions | 主界面「会话」→ 新建会话对话框 | 已实现 |
| 会话生命周期 | sessions | 会话卡片状态（idle/starting/running/failed/stopped） | 已实现 |
| 会话分类 | sessions | 会话侧边栏分组 | 已实现 |
| 置顶与归档 | sessions | 会话卡片操作 | 已实现 |
| 会话导出 | sessions | 会话操作 | 已实现 |
| 聊天配置 | sessions | 会话内配置（权限模式、provider、模型、推理深度、流式、思考、长上下文） | 已实现 |
| 推理深度自动钳制 | sessions | 换模型时自动下钳，无提示 | 已实现 |
| 工作区标签页（9 个） | sessions | 会话顶部：**工作区 / 变更 / 文档 / 文件 / 终端记录 / Shell / 日志 / 链路 / 报告** | 已实现 |
| Agent CLI 工作区 | agent_runtime | 「工作区」标签内的 Agent 终端 | 已实现 |
| 交互式 Shell | workspaces | 「Shell」标签 | 已实现 |
| 文件浏览与预览 | workspaces | 「文件」标签（>1 MiB 或二进制不预览） | 部分实现（有预览上限） |
| Git 变更查看 | workspaces | 「变更」标签（统一/分栏视图） | 已实现 |
| 会话报告 | sessions | 「报告」标签（Token 分布、工具排行、时间线） | 已实现 |

### 2.2 多 Agent 协作

| 功能名 | 所属模块 | 入口位置 | 实现状态 |
|---|---|---|---|
| 多 Agent 群聊席位 | agent_runtime | 新建会话对话框 → 席位分配 | 已实现 |
| `@` 交接发言权 | agent_runtime | 对话输入 `@` 触发席位补全 | 已实现 |
| 交回人类（三种意图） | agent_runtime | Agent 回复中 `@用户` / `@用户 handoff` / `@用户 done` | 已实现 |
| 席位切换视图 | sessions | 会话工作区席位切换器 | 已实现 |
| 发言人标注 | sessions | 消息列表 | 已实现 |
| Loop 工程化运行时 | agent_runtime | 活动栏「**循环工程**」 | 已实现 |
| Loop 人工验收 | agent_runtime | 运行进入 `awaiting-acceptance` 后需确认 | 已实现 |
| ~~DAG 依赖图编排~~ | — | — | **不存在（已移除）** |

### 2.3 权限与治理

| 功能名 | 所属模块 | 入口位置 | 实现状态 |
|---|---|---|---|
| 授权模板（四档） | permissions | 设置 →「**Agent 权限策略**」 | 已实现 |
| 审批弹窗与作用域 | permissions | 执行中命中 `Ask` 时弹出（本次/本会话/本项目/全局） | 已实现 |
| 审计记录 | permissions | 落库，每次已解析判定均写入 | 已实现 |
| Claude Code 权限钩子 | permissions | 自动安装，含离线降级 | 已实现 |
| 提权二次确认 | permissions | 切到 Trusted / Yolo 时要求确认 | 已实现 |
| 委派（delegation） | permissions | — | **仅有预留，未启用** |

### 2.4 个性化

| 功能名 | 所属模块 | 入口位置 | 实现状态 |
|---|---|---|---|
| Custom Instructions | agent_runtime | 设置 →「**个性化**」→ 关于你 / 风格规则 | 已实现 |
| Agent 记忆 | agent_runtime | 设置 →「**个性化**」→ 记忆区 | 已实现（主机级共享池，无法按 Agent 隔离） |
| 记忆开关 | agent_runtime | 同上（含「工具辅助对话」单独开关） | 已实现 |
| 专家角色 | agent_runtime | 设置 →「**专家角色**」（3 个内置 + 自定义） | 已实现 |
| 角色评审策略 | agent_runtime | 角色表单（可作评审 / 要求异模型族） | 已实现 |

> **重要前提**：CLI Agent 的记忆提取由 OnePiece 代做——**未配置 OnePiece provider 时，CLI Agent 不产生记忆**。这是用户会实际撞上但界面看不出来的约束，必须写进文档。

### 2.5 工具生态

| 功能名 | 所属模块 | 入口位置 | 实现状态 |
|---|---|---|---|
| MCP 服务器管理 | tooling/mcp | 设置 →「**MCP 服务器**」 | 已实现 |
| MCP 配置导入 | tooling/mcp | MCP 页导入 | 已实现 |
| MCP 中继 | tooling/mcp | 自动，**仅对 claude-code 与 codex-cli 启用** | 部分实现（覆盖 2/4 CLI） |
| Skill 管理与漂移检测 | tooling/skills | 设置 →「**Skill 管理**」 | 已实现 |
| Prompt Hook | tooling/prompt_hooks | 设置 →「**Prompt Hook**」 | 已实现（不作用于 OnePiece） |
| 本地扩展（OCR/ASR/TTS） | tooling/extensions | 设置 →「**扩展能力**」 | 已实现 |
| 插件集成 | tooling/plugin_integrations | 设置 →「**插件集成**」 | 已实现 |
| SDK 依赖管理 | tooling/sdk | 设置 →「**Agent 配置**」相关 | 已实现（仅 claude-sdk / codex-sdk 两个） |
| CLI 检测与冲突处理 | tooling/cli | 设置 →「**CLI 管理**」 | 已实现 |
| CLI 启动参数 | tooling/cli_parameters | 设置 →「**CLI 参数**」 | 已实现 |

### 2.6 工作区与远程

| 功能名 | 所属模块 | 入口位置 | 实现状态 |
|---|---|---|---|
| 项目目录选择 | workspaces | 新建会话 → 工作区区块 | 已实现 |
| 工作区历史 | workspaces | 工作区选择器 | 已实现 |
| Git worktree | workspaces | 工作区区块 | 已实现（**远端不支持 worktree**） |
| 命令模板 | workspaces | 三级作用域 | 已实现 |
| 终端输出检索 | workspaces | 「终端记录」标签 | 已实现（保留 30 天 / 512 MiB） |
| 文件夹打开器 | desktop | 设置 →「基础配置」相关区 | 已实现 |
| SSH 连接 | ssh_connections | 设置 →「**SSH 连接**」 | 已实现 |
| 远程终端 | ssh_connections | 远程工作区下的终端标签 | 已实现（并发上限 8） |
| IM 连接器（5 个） | communications | 设置 →「**IM 能力**」 | 已实现（飞书/钉钉/企微/微信/Telegram） |

### 2.7 自动化与洞察

| 功能名 | 所属模块 | 入口位置 | 实现状态 |
|---|---|---|---|
| 定时任务 | sessions | 活动栏「**定时任务**」 | 已实现（五种频率；应用退出后不触发） |
| 通知中心 | 前端 | 通知中心与 toast | 已实现 |
| 用量统计 | agent_runtime | 设置 →「**使用统计**」 | 已实现（口径来自各 CLI 自述） |
| 执行可观测性（链路） | execution_observability | 会话「**链路**」标签 + 设置 →「**执行可观测性**」 | 已实现 |
| 会话日志查看 | 前端 | 会话「**日志**」标签（四级 + 搜索 + 定位 + 导出） | 已实现（Web 预览模式不支持导出） |
| 原生 API Agent（OnePiece） | agent_runtime | 设置 →「**Agent 配置**」→ OnePiece 面板 | 已实现（25 家 provider） |
| 记忆检索（recall） | retrieval | 设置 → OnePiece 配置内的检索区 | 已实现（**只索引记忆，不索引项目代码**） |

### 2.8 桌面与系统

| 功能名 | 所属模块 | 入口位置 | 实现状态 |
|---|---|---|---|
| 基础配置 | desktop | 设置 →「**基础配置**」 | 已实现 |
| 界面语言（5 种） | 前端 | 基础配置 → 语言（简中/英/繁中/日/韩） | 已实现 |
| 悬浮助手 | desktop | 设置 → 悬浮助手区 | 已实现 |
| 开机自启 | desktop | 设置 → 启动项区 | 已实现 |
| 网络代理 | desktop | 设置 → 网络代理区 | 已实现 |
| 数据管理 | desktop | 设置 → 数据管理区 | 已实现 |
| 关于 | desktop | 设置 →「**关于**」 | 已实现 |

---

## 三、明确不写入正式文档的内容

| 项 | 原因 |
|---|---|
| DAG 工作流 / 拓扑可行性 | 能力已被移除（迁移 45） |
| 检查点（Checkpoint） | 无此用户功能 |
| Context Handoff / 跨 CLI 会话迁移 | 仅存在于未合并分支 |
| 权限委派（delegation） | 列已预留但强制为空 |
| `L3` 风险等级 | 已声明但当前不产生 |
| 资源级授权规则 | `ResourcePattern::Exact` 不由任何模板构造 |
| benchmark / eval 基础设施 | 未在 `main` 的 specs 或界面中找到 |

---

## 四、需要你决策的问题

1. **目录落点**：写进现有 mdBook 的 `docs/user-guide/zh-CN/src/`（受双语对等约束），还是另起一套不受该规范管辖的目录？
2. **英文版**：规范要求双语对等。是同步写英文版，还是先起 OpenSpec change 说明分期交付？
3. **截图**：接受"先不放图片链接、只维护 `_screenshot-checklist.md`"（CI 可过），还是同步补 Playwright 场景真出图？
4. **与 `docs/zh/` 的关系**：两套中文文档如何互链与分工？
5. **多 Agent 章节**：确认改写为「群聊席位 + `@` 交接」与「Loop 工程化」两节？
6. **使用案例**：第 2、4 条依赖不存在的功能，是否替换为 Loop 流水线与 IM/定时任务自动化？

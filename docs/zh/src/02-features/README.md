# 功能总览

> 本章面向**使用者**：讲清 VaneHub AI 有哪些能力、各自解决什么问题、从哪个入口进入。架构层面的实现细节在 [03-architecture](../03-architecture/README.md)。

## 怎么读这组文档

**每篇功能文档结构一致**：功能定位 → 使用场景 → 能力清单 → 核心模型 → 使用方式 → 边界与限制。

想快速判断某个能力能不能满足需求，直接看**能力清单**（含运行时适用范围）和**边界与限制**两节。

## 状态标签说明

| 标签 | 含义 |
|---|---|
| **已交付** | 有已归档的 OpenSpec 变更，且存在用户可见的界面入口 |
| **预览** | 服务契约与原生实现存在，但用户可见入口尚未开放或仅部分开放 |
| **计划中** | 仅有规范或分支留痕，`main` 上不可用 |

**本表状态的依据**是三项证据的交集：`openspec/changes/archive/archive-index.json` 中的已归档变更（116 条）、`openspec/specs/` 下的能力规范（88 个）、以及 `src/` 中实际存在的界面入口文件。任何一项缺失都会在备注里写明。

## 功能矩阵

### 会话与执行

| 功能 | 说明 | 状态 | 依据 |
|---|---|---|---|
| **多 CLI 会话管理** | 统一的会话创建、分类、置顶、归档、导出；一个会话绑定一个或多个 Agent | 已交付 | `session-management`、`session-category-management`、`session-export`；入口 `src/main-layout/create-session-dialog.tsx` |
| **聊天配置** | 权限模式、provider、模型、推理深度、流式、思考、长上下文 | 已交付 | `session-chat-configuration`、`agent-chat-configuration`；`sessions/domain/chat_configuration.rs` |
| **交互式 Agent 终端** | 基于 PTY 的真实终端交互，含命令管理与输出搜索 | 已交付 | `agent-terminal-runtime`、`terminal-command-management`、`terminal-output-search`、`session-shell` |
| **工作区标签页** | 会话内 9 个标签：对话、变更、文档、文件、终端、shell、日志、追踪、报告 | 已交付 | `session-workspace-tabs`；`src/session-workspace/session-tab-bar.tsx:19-28` |
| **多 Agent 群聊** | 一个会话内多个 Agent 席位，通过 `@` 交接发言权 | 已交付 | `multi-agent-group-chat`；入口 `src/main-layout/session-seat-assignment.tsx`。**取代了已被移除的 `multi-agent-coordination`**（迁移 45 `remove-multi-agent-coordination` 删除了 `coordination_runs` 表） |
| **Loop 工程化运行时** | 目标驱动的自动循环：编排、执行、校验、恢复；强制人工验收 | 已交付 | `loop-engineering-runtime`、`loop-management-ui`；入口 `src/loop-center/` |
| **原生 API Agent** | 内置 OnePiece Agent，直连 25 家模型 provider | 已交付 | `onepiece-native-agent`、`api-agent-runtime`、`native-model-discovery` |

详见：[会话管理](session-management.md)、[多 Agent 群聊](group-chat.md)、[Loop 工程化](loop-engineering.md)、[原生 API Agent](native-agent.md)

### 治理与个性化

| 功能 | 说明 | 状态 | 依据 |
|---|---|---|---|
| **权限审批** | 高风险操作的拦截、审批、四档授权模板与审计记录 | 已交付 | `permissions-core`、`permissions-approval`、`agent-tool-trust`；入口 `src/settings/pages/agent-policies-page.tsx` |
| **Claude Code 权限钩子** | 以独立二进制介入 Claude Code 的权限回调，含离线降级 | 已交付 | `claude-code-permission-hook`；`src-tauri/src/bin/vanehub-permission-hook.rs` |
| **CLI 权限启动参数** | 其余三个 CLI 通过启动参数/环境变量表达权限模板 | 已交付 | `cli-agent-permission-launch-flags`；`providers/invocation.rs:7-12` |
| **Custom Instructions** | 「关于你」与「风格规则」两段统一注入的指令 | 已交付 | `custom-instructions`；入口 `src/settings/pages/personalization-page.tsx` |
| **跨会话记忆** | 主机级共享记忆池，落 `agent_memories` 表 | 已交付 | `agent-cross-session-memory`；迁移 42 `agent-memory-shared-pool` |
| **上下文压缩** | 长对话的短期上下文压缩，压缩时顺带提取记忆 | 已交付 | `agent-context-compaction` |
| **专家角色** | 3 个内置角色 + 自定义，带职责、技能绑定与评审策略 | 已交付 | `expert-role-management`；入口 `src/settings/pages/expert-roles-page.tsx` |

详见：[权限审批](agent-permission.md)、[个性化](personalization.md)

### 工具生态

| 功能 | 说明 | 状态 | 依据 |
|---|---|---|---|
| **MCP 客户端管理** | 三种传输、两级作用域的 server 注册与连接 | 已交付 | `mcp-client-management`、`agent-mcp-tools`；入口 `src/settings/pages/mcp-page.tsx` |
| **MCP 中继** | 把统一注册的 server 暴露给外部 CLI（当前限 2 个 Agent） | 已交付 | `mcp-client-management`；`bootstrap/managed_mcp_relay.rs:110` |
| **工具注册与执行** | 统一的工具目录、信任级别与调用 | 已交付 | `agent-tool-registry`、`agent-tool-execution` |
| **Skill 管理** | Skill 安装、绑定到 Agent、六类漂移检测 | 已交付 | `skill-management`、`agent-skill-injection`；入口 `src/settings/pages/skills-page.tsx` |
| **Prompt Hooks** | 六类分类、两个执行阶段的提示词钩子 | 已交付 | `prompt-hook-management`；入口 `src/settings/pages/prompt-hooks-page.tsx` |
| **本地扩展** | OCR / ASR / TTS 三类本地 AI 能力 | 已交付 | `local-extension-management`；入口 `src/settings/pages/extensions-page.tsx` |
| **插件集成** | 第三方插件集成配置 | 已交付 | `plugin-integration-management`；入口 `plugin-integrations-page.tsx` |
| **SDK 依赖管理** | 两个受管 SDK 的安装与版本管理 | 已交付 | `sdk-dependency-management`；入口 `src/settings/pages/sdk-page.tsx` |
| **CLI 配置与参数** | 安装检测、环境、四类冲突处理与启动参数 | 已交付 | `cli-agent-config-management`、`cli-parameter-management`；入口 `cli-parameters-page.tsx` |

详见：[工具生态](tooling.md)

### 工作区与远程

| 功能 | 说明 | 状态 | 依据 |
|---|---|---|---|
| **项目与 worktree** | 项目探测、Git worktree 管理（含 Loop 专用） | 已交付 | `project-worktree-management`、`session-project-inspection` |
| **路径安全边界** | 两层防逃逸：语法校验 + 规范化边界 | 已交付 | `workspaces/domain/path.rs` |
| **shell 终端与输出捕获** | 有界队列、缺口标记、分页检索、30 天保留 | 已交付 | `session-shell`、`terminal-output-search`；`workspaces/domain/remote_terminal_limits.rs` |
| **命令模板** | 三级作用域的可复用命令，快照不可变 | 已交付 | `terminal-command-management` |
| **文件夹打开器** | 用外部编辑器/文件管理器打开工作区 | 已交付 | `workspace-folder-openers`；入口 `folder-openers-section.tsx` |
| **SSH 连接** | 主机密钥 TOFU 校验、凭据安全存储 | 已交付 | `ssh-connection-management`；入口 `ssh-connections-page.tsx` |
| **远程终端** | 连接池上限 8、空闲 5 分钟回收 | 已交付 | `remote-terminal-runtime` |
| **IM 连接器** | 飞书、钉钉、企业微信、微信、Telegram | 已交付 | `im-connector-management`；入口 `src/settings/pages/im-page.tsx` |

详见：[项目与工作区](workspaces.md)、[远程与 IM](remote-and-im.md)

### 自动化与洞察

| 功能 | 说明 | 状态 | 依据 |
|---|---|---|---|
| **定时任务** | 五种频率；本地时区算下次、UTC 判到期 | 已交付 | `scheduled-task-management`；入口 `scheduled-tasks-dialog.tsx`，实现在 `sessions` 上下文 |
| **长时操作跟踪** | SDK/MCP/Agent/工作区/扩展五类操作的排队、状态与日志 | 已交付 | `operations` 上下文；`src/services/operation-service.ts` |
| **通知** | 四类通知，支持全局与会话作用域 | 已交付 | `notification-system`；`src/notifications/` |
| **用量统计** | 四维 token（含缓存读/写），四条独立摄取路径 | 已交付 | `usage-statistics`；入口 `usage-statistics-page.tsx` |
| **执行可观测性** | 四级 Span 树、四种保真度、两级脱敏 | 已交付 | `agent-execution-observability`；入口 `observability-settings-page.tsx`、`execution-timeline-tab.tsx` |
| **统一日志** | 四级日志、统一目录、落盘前 token 级脱敏 | 已交付 | `unified-log-management`、`session-log-viewer`；入口 `log-management-section.tsx` |
| **记忆检索（recall）** | 对记忆池的向量 + 关键词混合检索，RRF 融合 | 已交付 | `retrieval-vector-search`；入口 `settings/pages/agents/onepiece-retrieval-section.tsx` |
| **桌面集成** | 悬浮助手、托盘、开机自启、网络代理、后台生命周期 | 已交付 | `desktop-floating-assistant`、`desktop-startup-controls`、`desktop-background-lifecycle`；`src/floating-assistant/` |

详见：[自动化与洞察](automation-and-insight.md)、[可观测性](observability.md)、[原生 API Agent](native-agent.md)

## 几处容易误解的地方

**这些是读代码时确认过、但从功能名称上看不出来的事实：**

| 常见理解 | 实际情况 |
|---|---|
| 「向量检索」= 对项目代码建索引 | **只索引 `agent_memories`**——`SourceKind` 当前唯一变体是 `AgentMemory`（`retrieval/domain/document.rs:4-8`） |
| 记忆按 Agent 隔离 | **主机级共享池**，迁移 42 之后读取不再按 `agent_id`/`folder` 过滤 |
| CLI Agent 自己提取记忆 | **由 OnePiece 代劳**；未配置 OnePiece provider 则不产生记忆（`memory_extraction_gateway.rs:12-19`） |
| 「本地扩展」= 通用插件 | **是 OCR / ASR / TTS 三类本地 AI 框架**（PaddleOCR、faster-whisper、sherpa-onnx） |
| 定时任务属于 `operations` 上下文 | **实现在 `sessions`**，因为它的产物是会话 |
| `Trusted` 比 `Yolo` 更严格 | **两者策略规则完全相同**，只在赋予时的确认强度上有别（`template.rs:51-57`） |
| `Readonly` 模板禁止一切 | **`file.read` 与 `memory.write` 恒为放行**，模板只区分 `shell.exec` 与 `file.write` |

## 功能与限界上下文的对应

**每个功能域背后对应一个或多个原生限界上下文**（`src-tauri/src/contexts/mod.rs:3-13`）：

| 限界上下文 | 承载的功能 |
|---|---|
| `sessions` | 会话、分类、消息、聊天配置、工作区标签页、导出、**定时任务** |
| `agent_runtime` | Agent 目录、CLI 进程、终端、群聊席位、Loop、记忆、个性化、专家角色、原生 API Agent |
| `permissions` | 权限审批、授权模板、审计、Claude Code 钩子桥接 |
| `tooling` | MCP、Skills、扩展、插件、Prompt Hooks、SDK、CLI 配置与参数（8 个子域） |
| `workspaces` | 项目、worktree、路径边界、shell 与输出捕获、命令模板、文件夹打开器 |
| `ssh_connections` | SSH 连接与远程终端运行时 |
| `communications` | 五个 IM 连接器 |
| `operations` | 长时操作的排队、状态与日志 |
| `execution_observability` | 执行追踪、Span 存储、采集策略、保留清理 |
| `retrieval` | 记忆池的混合检索（服务于原生 API Agent） |
| `desktop` | 设置、悬浮助手、托盘、启动项、网络代理、前端日志上报 |

## 已修正的文档陈述过时点

**撰写本文档集时，根 README 的 Feature status 一节有三处已落后于代码，现已随本次改动一并修正：**

| 原先的说法 | 实际情况 | 依据 |
|---|---|---|
| "Planned: The normal create-session UI still disables Multi Agent mode" | 多 Agent 群聊已合入 `main`，创建会话对话框已挂载席位分配组件 | commit `d104027`；`src/main-layout/create-session-dialog-content.tsx:158` |
| "Planned: Japanese runtime UI resources... not for the application UI" | **日语 UI 已完整支持** | `ja` 已注册进 `supportedLocales`（`src/i18n/supported-locales.ts:32`）；五种语言资源**键数完全一致（各 2197 条）** |
| "Preview: Multi-Agent coordination has native and Web/mock service contracts..." | **该运行时已被移除**，由群聊取代 | 迁移 45 `remove-multi-agent-coordination` 执行 `DROP TABLE coordination_runs`；`src/services`、`src/contracts`、`src-tauri/src/contexts` 中已无任何 coordination 引用 |

**第三处最值得留意**：它描述的不是"尚未做完"，而是**一个做过又被撤掉的能力**。归档区仍保留 `multi-agent-coordination` 的两条变更记录，但那是历史，不是现状——这正是 [OpenSpec 工作流](../04-development/openspec-workflow.md#使用文档中的陷阱) 里"代码 > 主 specs > 归档 > README"这条优先级的由来。

## 运行时差异提醒

**Web/mock 模式不具备原生能力**。以下功能依赖桌面运行时，在浏览器模式下不可用或仅为模拟数据：

CLI 进程启动、PTY 终端、SQLite 持久化、文件系统访问、SSH 连接、IM 连接、系统凭据存储、桌面通知与托盘、开机自启、权限拦截、执行追踪采集。

**各篇文档的「能力清单」表格都带「运行时」列**，逐项标注了适用范围。

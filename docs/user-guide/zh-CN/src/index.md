# VaneHub AI 用户指南

<a href="../../en/src/index.md">English</a>

VaneHub AI 是统一运行、管理和协作多个 AI 编码 Agent 的桌面工作台：既能驱动 Claude Code、Codex CLI 等外部 CLI，也内置不依赖任何 CLI 的原生 Agent OnePiece。

本指南面向**使用 VaneHub AI 完成软件开发工作的个人与团队**。参与 VaneHub AI 本身的开发、测试或贡献代码，请阅读[开发者指南](../../../developer-guide/zh-CN/src/index.md)。

> 本页按常见任务提供入口，完整章节列表请查看左侧目录。

## 第一次使用

先选一种执行方式：

| 使用方式 | 适合谁 | 推荐路径 |
| --- | --- | --- |
| **OnePiece（原生 Agent）** | 不想安装任何编码 CLI | 安装应用 → 配置模型服务 → 创建会话 → 发送任务 |
| **外部 CLI Agent** | 已在用 Claude Code、Codex CLI、Gemini CLI、OpenCode 或 Antigravity CLI | 安装应用 → 安装并认证 CLI → 检查检测状态 → 创建会话 |

两条路都从这些章节开始：

- **下载安装**：预构建安装包与校验方式见 [README 的「下载」一节](../../../../README.zh-CN.md#下载)
- [快速开始](quick-start.md)——最短成功路径，五分钟发出第一条任务
- [安装并认证 CLI](getting-started.md)——走外部 CLI 路线时的安装、认证与检测（OnePiece 路线可跳过）
- [OnePiece（原生 Agent）](native-agent.md)——走 OnePiece 路线时的模型服务配置
- [创建第一个会话](first-session.md)——选 Agent、选工作区、认识会话工作区
- [用户界面](user-interface.md)——主窗口布局、会话列表、对话区、浮动助手、通知与托盘
- [会话工作区](session-workspace.md)——会话里的对话、变更、文件、Shell、日志与链路各区域
- [设置中心](settings.md)——全局设置入口与各设置页导航
- [核心概念](core-concepts.md)——会话、席位、工作区、权限模板、Loop、MCP 这些词各指什么

## 按目标查找

### 界面与工作区

| 我需要…… | 从这里开始 | 相关主题 |
| --- | --- | --- |
| 认识主窗口与各功能区 | [用户界面](user-interface.md) | [创建第一个会话](first-session.md) |
| 在会话里看对话、变更、Shell、日志与链路 | [会话工作区](session-workspace.md) | [创建第一个会话](first-session.md#会话工作区的九个标签页) |
| 配置应用本身（语言、主题、代理、数据目录） | [设置中心](settings.md) | [版本更新](app-updates.md) |
| 让 Agent 在独立工作副本里改代码 | [Git Worktree](worktree.md) | [创建第一个会话](first-session.md) |
| 在远程机器上工作 | [远程工作区与 SSH](remote-workspaces.md) | — |

### Agent 与协作

| 我需要…… | 从这里开始 | 相关主题 |
| --- | --- | --- |
| 不装 CLI 直接用内置 Agent | [OnePiece（原生 Agent）](native-agent.md) | [快速开始](quick-start.md) |
| 安装、认证与管理外部 CLI | [安装并认证 CLI](getting-started.md) | [Agent 与 CLI 配置](agent-configuration.md) |
| 多个 Agent 在一个会话里协作 | [多 Agent 群聊](multi-agent-workflow.md) | [专家角色](expert-roles.md) |
| 逐行评审 Agent 的改动 | [代码评审](code-review.md) | — |
| 比较不同 Agent 的任务表现 | [Agent 评测](evaluation.md) | — |

### 上下文与代码智能

| 我需要…… | 从这里开始 | 相关主题 |
| --- | --- | --- |
| 让 Agent 跨会话记住事情、理解上下文压缩 | [记忆与上下文](memory-and-context.md) | [个性化](personalization.md) |
| 设置回复风格与关于我的信息 | [个性化](personalization.md) | [专家角色](expert-roles.md) |
| 为工作区建代码索引 | [代码索引](code-indexing.md) | [LSP 代码智能](lsp-code-intelligence.md) |
| 会话内符号跳转与诊断 | [LSP 代码智能](lsp-code-intelligence.md) | — |

### 工具与集成

| 我需要…… | 从这里开始 | 相关主题 |
| --- | --- | --- |
| 给 Agent 接上外部工具 | [MCP 服务器](mcp.md) | [Agent 与 CLI 配置](agent-configuration.md) |
| 安装 Skill 并绑定到 Agent | [Skill 管理](skill-management.md) | — |
| 在提示词组装链路里插入内容 | [Prompt Hook](prompt-hooks.md) | [Agent 与 CLI 配置](agent-configuration.md) |
| 本地 OCR、语音识别与语音合成 | [本地媒体](local-media.md) | [本地扩展](extensions.md) |
| 连接 GitHub 等外部产品 | [插件集成](plugin-integration.md) | — |
| 从飞书、钉钉等 IM 触发会话 | [IM 连接器](im-connectors.md) | — |
| 在输入框里直接切页签、改开关 | [斜杠命令](slash-commands.md) | — |

### 治理与运行

| 我需要…… | 从这里开始 | 相关主题 |
| --- | --- | --- |
| 控制 Agent 能做什么、处理审批 | [权限审批](permissions.md) | — |
| 让 Agent 自动迭代到目标达成 | [Loop Engineering 工程](loop-engineering.md) | [目标与任务看板](goals-and-work-board.md) |
| 追踪目标与待办 | [目标与任务看板](goals-and-work-board.md) | — |
| 按周期自动跑任务 | [定时任务与通知](scheduled-tasks.md) | — |
| 查看 token 用量 | [使用统计](usage-statistics.md) | — |
| 查看执行链路与日志 | [可观测性](observability.md) | [故障排查](troubleshooting.md) |
| 更新应用版本 | [版本更新](app-updates.md) | — |

## 功能可用性

部分功能有平台或依赖约束：例如插件集成仅在桌面端可用，外部 CLI Agent 需要对应 CLI 已安装并认证，OnePiece 与记忆提取需要配好模型服务，本地媒体需要你自备模型文件。各功能页末尾的「注意事项与限制」一节说明该功能的具体约束。

## 获得帮助

| 情况 | 去哪里 |
| --- | --- |
| 不知道某项功能怎么操作 | [常见问题](faq.md) |
| 应用或 Agent 运行异常 | [故障排查](troubleshooting.md) |
| 确认存在缺陷、安全问题，或想提功能建议 | [反馈问题与提交 Issue](reporting-issues.md) |
| 想看端到端的完整操作场景 | [使用案例](use-cases.md) |

## 文档范围

本指南只讲用户能做什么、如何操作、如何判断结果、失败后如何恢复。想了解架构如何切分、某个机制为什么这样设计、代码在哪，见[开发者指南](../../../developer-guide/zh-CN/src/index.md)；想学习 MCP、LSP、RAG 这些技术本身，见 [Agent 基础设施技术文档](../../../agent-infrastructure/README.md)。

## 英文版

英文版与中文版采用相同的整体目录结构，个别章节的翻译进度可能存在短暂差异。见 <a href="../../en/src/index.md">English</a>。

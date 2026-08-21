# VaneHub AI 用户指南

<a href="../../en/src/index.md">English</a>

本指南面向**使用 VaneHub AI 的开发者**，讲清怎么装、怎么用、出问题怎么查。

## 上手五步

| 顺序 | 章节 | 讲什么 |
| --- | --- | --- |
| 1 | [快速开始](quick-start.md) | 装好、跑起来、发出第一条消息 |
| 2 | [安装并认证 CLI](getting-started.md) | 五个 CLI 的安装与认证，可用性怎么看 |
| 3 | [创建第一个会话](first-session.md) | 选 Agent、选工作区、理解会话工作区的九个标签页 |
| 4 | [核心概念](core-concepts.md) | 会话、席位、工作区、权限、Loop、MCP 这几个词各指什么 |
| 5 | [用户界面](user-interface.md) | 逐个功能过一遍界面上能做什么 |

赶时间只读第 1 篇即可，其余按需回来查。

## 功能详解

| 章节 | 讲什么 |
| --- | --- |
| [多 Agent 群聊](multi-agent-workflow.md) | 一个会话里放多个 Agent，用 `@` 交接发言权 |
| [群聊协作案例](multi-agent-testing-tutorial.md) | 用架构师、实现者和代码审查走完 UI、交接与历史身份验收 |
| [Git Worktree](worktree.md) | 让 Agent 在独立工作副本里改代码，不动你的分支 |
| [Loop Engineering 工程](loop-engineering.md) | 定目标与必过检查，让它自动迭代到达成 |
| [目标管理](goal-management.md) | 把计划、循环与看板项归到一个目标下追踪 |
| [任务看板](todo-board.md) | 人工待办与 Agent 工作放在同一块看板 |
| [斜杠命令](slash-commands.md) | 在输入框里直接切页签、改开关、看用量 |
| [代码评审](code-review.md) | 逐行看差异、留评论、把意见送回 Agent |
| [记忆与上下文](memory-and-context.md) | 跨会话记住什么，以及上下文满了怎么压缩 |
| [权限审批](permissions.md) | 四档模板、审批弹窗、记忆范围 |
| [个性化](personalization.md) | 关于你、回复风格、跨会话记忆、专家角色 |
| [Skill 管理](skill-management.md) | 安装 Skill、绑定到 Agent、漂移提示、演进证据 |
| [代码索引](code-indexing.md) | 工作区代码的向量索引 |
| [LSP 代码智能](lsp-code-intelligence.md) | 会话内的符号跳转与诊断 |
| [工具与扩展](tooling.md) | MCP 服务器、Prompt Hook、本地 OCR/语音扩展 |
| [MCP 服务器](mcp.md) | 给 Agent 接上外部工具，以及逐次工具审批 |
| [Prompt Hook](prompt-hooks.md) | 在提示词组装链路里插入内容，草稿/发布/回滚 |
| [OnePiece（原生 Agent）](native-agent.md) | 不装 CLI 也能用，配 provider、recall 与 Notebook 编辑 |
| [可观测性](observability.md) | 执行链路、保真度、日志目录与脱敏 |
| [远程与 IM](remote-and-im.md) | SSH 远程工作区、飞书/钉钉/企微/微信/Telegram |
| [定时任务与用量统计](automation.md) | 按周期自动跑任务，以及 token 用量怎么看 |
| [版本更新](app-updates.md) | 发布通道、签名校验与自动更新 |

## 参考

| 章节 | 讲什么 |
| --- | --- |
| [使用案例](use-cases.md) | 五个端到端场景，从头走一遍 |
| [常见问题](faq.md) | 高频疑问的直接回答 |
| [故障排查](troubleshooting.md) | 出错了先看这里 |

## 状态标签

- **已实现** —— 用户可见路径已经实现并经过验证。
- **预览** —— Service 或 mock contract 已存在，但正常产品工作流尚未完成。
- **仅桌面端** —— 依赖 Tauri runtime 与本地操作系统访问能力。
- **规划中** —— 当前尚不可用。

## 本指南不讲什么

**内部实现与设计动机不在这里**。想了解架构如何切分、某个机制为什么这样设计、代码在哪，见[《VaneHub AI 开发者指南》](../../../developer-guide/zh-CN/src/index.md)——它面向开发者与贡献者，架构决策记录在 [Native 架构清单](../../../developer-guide/zh-CN/src/index.md) 指向的 `src-tauri/ARCHITECTURE.md`。

两者分工：

| 想知道 | 去哪 |
| --- | --- |
| 这个按钮点了会怎样 | 本指南 |
| 出错了怎么办 | 本指南的[故障排查](troubleshooting.md) |
| 这个功能为什么这样设计 | [开发者指南](../../../developer-guide/zh-CN/src/index.md) |
| 代码在哪个文件 | [开发者指南](../../../developer-guide/zh-CN/src/index.md) |
| MCP、LSP、RAG 这些技术本身是什么 | [Agent 基础设施技术文档](../../../agent-infrastructure/README.md) |
| 我发现了一个问题，怎么报 | 本指南的[反馈问题与提交 Issue](reporting-issues.md) |

## 英文版

英文用户指南已覆盖与本指南相同的全部章节。见 <a href="../../en/src/index.md">English</a>。

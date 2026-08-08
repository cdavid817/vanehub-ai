# VaneHub AI 用户指南

<a href="../en/index.html">English</a>

本指南面向**使用 VaneHub AI 的开发者**，讲清怎么装、怎么用、出问题怎么查。

## 上手四步

| 顺序 | 章节 | 讲什么 |
| --- | --- | --- |
| 1 | [快速开始](quick-start.md) | 装好、跑起来、发出第一条消息 |
| 2 | [安装并认证 CLI](getting-started.md) | 四个 CLI 的安装与认证，可用性怎么看 |
| 3 | [创建第一个会话](first-session.md) | 选 Agent、选工作区、理解会话工作区的九个标签页 |
| 4 | [核心概念](core-concepts.md) | 会话、席位、工作区、权限、Loop、MCP 这几个词各指什么 |

赶时间只读第 1 篇即可，其余按需回来查。

## 功能详解

| 章节 | 讲什么 |
| --- | --- |
| [多 Agent 群聊与 `@` 交接](multi-agent-workflow.md) | 一个会话里放多个 Agent，用 `@` 交接发言权 |
| [Loop 工程化](loop-engineering.md) | 定目标与必过检查，让它自动迭代到达成 |
| [权限审批](permissions.md) | 四档模板、审批弹窗、记忆范围 |
| [个性化配置](personalization.md) | 关于你、回复风格、跨会话记忆、专家角色 |
| [管理 Skill](skill-management.md) | 安装 Skill、绑定到 Agent、漂移提示 |
| [工具与扩展](tooling.md) | MCP 服务器、Prompt Hooks、本地 OCR/语音扩展 |
| [原生 API Agent](native-agent.md) | OnePiece：不装 CLI 也能用，配 provider 与 recall |
| [可观测性与日志](observability.md) | 执行链路、保真度、日志目录与脱敏 |
| [远程执行与 IM 接入](remote-and-im.md) | SSH 远程工作区、飞书/钉钉/企微/微信/Telegram |
| [定时任务与用量统计](automation.md) | 按周期自动跑，以及 token 用量怎么看 |

## 参考

| 章节 | 讲什么 |
| --- | --- |
| [使用案例](use-cases.md) | 五个端到端场景，从头走一遍 |
| [常见问题](faq.md) | 高频疑问的直接回答 |
| [Runtime 与功能状态标签](runtime-labels.md) | 「仅桌面端」「仅 Web/mock」这些标签怎么读 |
| [故障排查](troubleshooting.md) | 出错了先看这里 |

## 状态标签

- **已交付** —— 用户可见路径已经实现并经过验证。
- **预览** —— Service 或 mock contract 已存在，但正常产品工作流尚未完成。
- **仅 Web/mock** —— 确定性的浏览器行为，不会产生 native side effect。
- **仅桌面端** —— 依赖 Tauri runtime 与本地操作系统访问能力。
- **规划中** —— 当前尚不可用。

各标签的判读方式见 [Runtime 与功能状态标签](runtime-labels.md)。

## 本指南不讲什么

**内部实现与设计动机不在这里**。想了解架构如何切分、某个机制为什么这样设计、代码在哪，见[《VaneHub AI 架构与实现》](../../zh/index.html)——它面向开发者与贡献者。

两者分工：

| 想知道 | 去哪 |
| --- | --- |
| 这个按钮点了会怎样 | 本指南 |
| 出错了怎么办 | 本指南的[故障排查](troubleshooting.md) |
| 这个功能为什么这样设计 | [架构与实现](../../zh/index.html) |
| 代码在哪个文件 | [架构与实现](../../zh/index.html) |

## 英文版

英文用户指南目前覆盖较早的一部分章节，本轮新增内容尚未同步。见 <a href="../en/index.html">English</a>。

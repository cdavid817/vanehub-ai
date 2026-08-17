# 快速开始

**状态：已实现——桌面端。**

五分钟从零到第一次 Agent 执行。已经装好 CLI 的话，直接从第 2 步开始。

## 1. 准备一个 CLI

VaneHub AI **驱动已安装的 Coding Agent CLI**，本身不代管 Provider 认证。至少装一个：

```powershell
npm install -g @anthropic-ai/claude-code
```

先在普通终端里跑一次并完成登录，确认它能接受提示词：

```powershell
claude
```

其余 CLI（Codex CLI、Gemini CLI、OpenCode、Antigravity CLI）按各自官方说明安装。详见[安装并认证 CLI](getting-started.md)。

## 2. 确认 VaneHub AI 检测到它

打开**设置 → CLI 管理**，检查目标 CLI 的状态。

如果显示未检测到，通常是桌面应用可见的 `PATH` 与你终端里的不一致——见[故障排查](troubleshooting.md)。

## 3. 创建第一个会话

1. 点击**新建**。
2. **会话类型**选择**单 Agent**。
3. 在 **Agent** 区选中一个可用的 Agent。
4. **工作区**选择**本地**，在**项目文件夹**中选定目录（可用**浏览**，或从**最近打开项目**里选）。
5. 填写会话标题，点击**创建**。

会话创建后进入 `idle` 状态，可以开始对话了。

## 4. 在工作区里干活

![会话工作区，顶部 9 个标签页，右侧信息面板](assets/screenshots/session-workspace-zh-CN.png)

界面分三块：左侧**会话列表**、中间**工作区**、右侧**信息面板**（会话、CLI 工具、运行状态、本次模型、工作区路径）。

顶部有 9 个标签页：

| 标签 | 用途 |
| --- | --- |
| **工作区** | 主界面，与 Agent 对话并查看其 CLI 终端 |
| **变更** | 本次会话产生的 Git 变更 |
| **文档** / **文件** | 浏览工作目录 |
| **终端记录** | Agent 的工具执行记录 |
| **Shell** | 独立的交互式终端 |
| **日志** | 会话日志（可搜索、可按时间定位） |
| **链路** | 执行追踪 |
| **报告** | Token 用量与工具排行 |

在**工作区**标签的输入框里写下你的任务，**Enter 发送、Shift+Enter 换行**。

## 5. 接下来做什么

| 想做的事 | 去这里 |
| --- | --- |
| 让多个 Agent 在一个会话里协作 | [多 Agent 群聊与 `@` 交接](multi-agent-workflow.md) |
| 让 Agent 自己迭代到测试通过 | [Loop 工程化](loop-engineering.md) |
| 限制 Agent 能做什么 | [权限审批](permissions.md) |
| 让 Agent 记住你的偏好 | [个性化配置](personalization.md) |
| 理解各种术语 | [核心概念](core-concepts.md) |

## 注意事项

- **Provider 凭据始终保存在各 CLI 自己的存储中**，VaneHub AI 不会要求你输入 Provider 密码。
- **浏览器预览（Web/mock）不会执行任何本地命令**。界面看起来能操作，但不会启动进程、不写数据库。判断依据见 [Runtime 与功能状态标签](runtime-labels.md)。

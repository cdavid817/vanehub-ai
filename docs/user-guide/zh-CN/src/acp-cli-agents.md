# ACP CLI Agent 与 iFlow 历史兼容条目

除原有五个 CLI 之外，VaneHub AI 还通过 **Agent Client Protocol**（ACP）驱动六个编码 CLI：Qwen Code、Kimi Code CLI、Qoder CLI、CodeBuddy Code、GitHub Copilot CLI 与 Cursor Agent CLI。第七个条目 iFlow CLI 仅作为历史兼容的原生终端选项保留。本章只讲与原有五个的差异；创建会话、工作区标签、权限模板等其余部分完全一样。

## ACP 对你意味着什么

对原有五个 CLI，VaneHub 每轮启动一个新进程并读取其输出。对 ACP CLI，**程序在整个会话期间持续运行**，VaneHub 通过结构化协议与之通信。实际表现：

- **权限请求来自 CLI 本身**。agent 想执行命令或写文件时，对话里会出现审批卡片，与 Claude Code 一样。选择「允许一次」只批准这一次请求，你点的任何选项都不会被静默扩大为「总是」。
- **提问与计划也是卡片**。Cursor Agent CLI 可能在继续之前向你提问或提出计划；在对话中应答或拒绝即可。卡片 30 分钟无人应答、被关闭或本轮被取消，都视为拒绝。
- **取消先是协作式的**。取消会请求 agent 停止；若它几秒内不回应，VaneHub 只终止为本会话启动的那个进程。
- **token 用量显示为「不可用」**。这些 CLI 不经 ACP 上报用量，VaneHub 显示不可用而不是误导性的零。
- **若连接在本轮中途断开**，消息以「已中断、副作用未知」结束。VaneHub 绝不自行重发你的提示；请检查 agent 改了什么，需要时自己再发一次。

## 安装与登录

安装与登录遵循与原有五个相同的规则：VaneHub 只从下表列出的受审计来源安装，**登录一律在终端完成**，「设置 → CLI 管理」的检测绝不启动 ACP 会话或登录。由于这些 CLI 都没有文档化的可靠只读登录检查，其认证状态显示为**未知**；未知不等于已登录。创建会话前请先在终端跑一次 CLI，确认它能接受提示。

| Agent | 命令 | 应用内来源 | 说明 |
| --- | --- | --- | --- |
| Qwen Code | `qwen` | npm `@qwen-code/qwen-code` | 厂商的独立安装器内部可能回退到 npm；VaneHub 直接使用 npm 包 |
| Kimi Code CLI | `kimi` | npm `@moonshot-ai/kimi-code` | 旧的 Python/uv 安装会被检测并作为单独的安装报告；VaneHub 绝不迁移它。会话绑定在其开始时的发行形态上 |
| Qoder CLI | `qoder` | npm `@qoder-ai/qodercli` | 上游暂不支持 Windows arm64，界面会如实显示；旧命令名 `qodercli` 作为别名接受 |
| CodeBuddy Code | `codebuddy` | npm `@tencent-ai/codebuddy-code` | 为会话选择**账号环境**（国际、国内或 iOA）；它与界面语言无关 |
| GitHub Copilot CLI | `copilot` | npm `@github/copilot`；Windows 上另有 WinGet | 独立的 Copilot CLI，不是旧的 `gh copilot` 扩展。tools 与 reasoning 选项在进程启动时固定，因此每个会话有自己的进程 |
| Cursor Agent CLI | `agent` | 官方安装器，仅最新版 | `agent` 是常见程序名；只有安装路径或版本输出指向 Cursor 的那一份才被接受，其他一律报告为身份不匹配且绝不启动 |

每个 ACP CLI 也都有**原生终端**选项，在会话终端里打开 CLI 自己的交互界面。

「设置 → CLI 管理」的每张卡片上有两个显式操作：

- **检查连接**（仅限已安装的 ACP CLI）启动程序做一次协议握手后即释放。它显示协议版本、程序上报的名称与版本、是否提供会话恢复，以及它声明的登录方式。它不创建会话、不发送提示，也不代表已登录。
- **登录说明**在浏览器中打开厂商自己的文档。只会打开 HTTPS 文档链接，且只在你点击时打开；登录本身仍在终端完成。

## 第三方端点

Qwen Code 与 iFlow 可以接任意 OpenAI 兼容端点。不必手改 `~/.qwen/.env` 或 `~/.iflow/settings.json`：在**设置 → Agent 配置**里为这两个 Agent 新建配置并应用即可，见 [Agent 配置](agent-configuration.md)。其余五家 CLI 的程序本身没有这类设置，VaneHub 也不会假装能配。

## 恢复会话

只有当 CLI 声明支持加载会话、且 VaneHub 仍持有它在同一安装上记录的精确会话标识时，ACP 会话才会被恢复。否则会开启新的外部会话并保留本地历史。VaneHub 绝不从 CLI 自己的历史里猜「上一个会话」。

## 无人值守运行

定时任务与多 Agent 席位可以使用 ACP CLI，但无人值守运行没有人来应答审批卡片。这类运行遵循任务的无人值守策略：权限请求被拒绝并标记为需要干预，而不是自动批准。

## iFlow CLI（历史兼容）

iFlow 的官方服务与 API 已于 2026-04-17 关闭。VaneHub 为仍在用已安装 CLI 配合自定义 API 配置的用户保留一个**历史兼容**条目：

- 它在会话对话框的「历史兼容」分组下，需你显式启用；出现的任何地方都标有「历史兼容」。
- 只有**原生终端**可用：没有统一对话、没有多 Agent 席位、没有定时任务。
- VaneHub 不安装它、不提供登录、不导入或迁移其配置、不声称任何官方服务。

## 排障

- **卡片提示 agent 在本轮结束前停止**（`max_tokens`、`refusal`、`max_turn_requests`）：CLI 自行提前停止。这不是经验证的成功，继续前请检查结果。
- 开始对话时提示**「… 未登录」**：CLI 回答当前没有已登录账号。请在终端用该 CLI 登录后重试；VaneHub 不会代为登录。
- 打开原生终端时提示**「策略无法兑现」**：所选权限模板在该 CLI 上没有对应的启动旗标（目前只有 Qoder CLI 的「只读」模板，它没有 plan 模式）。换一个该 CLI 能兑现的模板；VaneHub 绝不自行回退到更宽松的模式。
- **Cursor 显示「身份不匹配」**：`PATH` 上找到的 `agent` 不是 Cursor 的。安装 Cursor 的 CLI 或调整 `PATH` 顺序；见[安装冲突](getting-started.md#安装冲突)。
- **CLI 参数页**：ACP CLI 与 iFlow 现在各有自己的条目（模型、推理强度或思考模式、Agent 档案，以及少量启动开关，均取自已安装程序的 `--help`）。权限旗标不在那里编辑：它们跟随权限模板，ACP 下只把只读姿态作为旗标传入。

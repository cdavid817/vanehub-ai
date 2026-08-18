# 安装并认证 CLI

**状态：已实现——桌面端设置。**

VaneHub AI **驱动你已经装好的 CLI**，不替你装模型、不代管 Provider 凭据。认证始终由各 CLI 自己完成，VaneHub AI 不会要求你输入 Provider 密码。

唯一的例外是 OnePiece——它是内置的原生 API Agent，不需要任何 CLI，API Key 由 VaneHub AI 保存。想跳过 CLI 直接开始，见[原生 API Agent](native-agent.md)。

## 前置条件

- Node.js 22+ 与 npm
- 至少一个受支持的 CLI，以及对应的订阅或 API 凭据

## 五个 CLI

VaneHub AI 支持五个外部 CLI Agent。装一个就能开始，不必五个都装。

| Agent | 提供方 | 命令 | npm 包 | 其他安装方式 |
| --- | --- | --- | --- | --- |
| Claude Code | Anthropic | `claude` | `@anthropic-ai/claude-code` | 安装脚本、winget（`Anthropic.ClaudeCode`） |
| Codex CLI | OpenAI | `codex` | `@openai/codex` | —— |
| Gemini CLI | Google | `gemini` | `@google/gemini-cli` | —— |
| OpenCode | OpenCode（开源） | `opencode` | `opencode-ai` | 安装脚本 |
| Antigravity CLI | Google | `agy` | 无 | 仅安装脚本（Unix `install.sh`、Windows `install.ps1`） |

### Claude Code

Anthropic 官方的命令行 AI 编程助手，VaneHub AI 里模型族归为 Anthropic。需要 Anthropic 订阅或 API 凭据。认证在普通终端里运行 `claude` 后按提示完成。VaneHub AI 通过 `claude-sdk` 或 PATH 上的 `claude` 判定其可用性，**不保存你的凭据**。

### Codex CLI

OpenAI 官方 CLI，模型族归为 OpenAI。需要 OpenAI 账号。认证在终端里运行 `codex` 后按提示登录，凭据由 Codex CLI 自行管理。

### Gemini CLI

Google 官方 CLI，模型族归为 Google。用 Google 账号认证，在终端里运行 `gemini` 后完成。凭据存在 Gemini CLI 自己的位置。

### OpenCode

开源 CLI（`opencode-ai`），支持多家 provider，模型族在 VaneHub AI 内归为 Unknown。认证方式随你选择的 provider 而定，在终端里运行 `opencode` 后配置。注意：OpenCode 不支持长上下文，VaneHub AI 会据此调整其上下文能力。

### Antigravity CLI

Google 官方 CLI，命令是 `agy`，模型族归为 Google。**没有 npm 包**，只能通过官方安装脚本安装（Unix `install.sh`、Windows `install.ps1`），因此 CLI 管理页对它不提供 npm 安装/升级/降级操作。它走 **Google 登录**并把凭据存进**系统钥匙串**，配置档里根本没有密钥字段。

> **凭据一律由各 CLI 自管**。VaneHub AI 只检测「这个命令能不能跑起来」，不替你走完登录，也不保存任何外部 CLI 的 Provider 凭据。

```powershell
npm install -g @anthropic-ai/claude-code
```

## 先在终端里跑通

**装完先在普通终端里运行一次并完成认证**，确认它能接受提示词：

```powershell
claude
```

这一步不能省。VaneHub AI 检测的是「这个命令能不能跑起来」，它无法替你走完各 Provider 的登录流程。**在终端里跑不通的 CLI，在 VaneHub AI 里也一样跑不通。**

## 读懂检测状态

设置中心 → CLI 工具页会显示每个 CLI 的状态。**六种，含义差别很大**：

| 状态 | 含义 | 该怎么办 |
| --- | --- | --- |
| **已安装** | 本机可解析到可执行文件 | 无需处理 |
| **未安装** | 本机未解析到可执行文件 | 按上表安装 |
| **已安装但不可运行** | 找到了文件，但执行失败 | 见下 |
| **安装冲突** | 检测到多处安装 | 见下 |
| **不支持** | 当前平台不支持该安装方式 | 改用其他来源 |
| **未检测** | 尚未执行检测 | 刷新检测 |

**「已安装但不可运行」不要靠重装解决**。界面上的提示写得很直接：

> 当前生效的 CLI 已安装但无法运行。请检查 Node、PATH 或该工具自身环境；重装同一版本通常不能直接解决。

问题通常出在 Node 版本、PATH，或该 CLI 自身的运行环境，而不是文件缺失。

## 安装冲突

**同一个 CLI 装了多份时会报「安装冲突」**——比如既用 npm 全局装过，又用安装脚本或 winget 装过一次。

点「诊断安装冲突」展开**安装诊断**，它会列出发现的所有本地安装路径，并标出**当前生效**的那一个。

界面的指引是：

> 检测到多处安装。请展开安装诊断确认当前生效路径；升级只应作用在命令行默认命中的那一处。

**为什么强调这一点**：升级错了那一份，命令行仍然命中旧的，看起来像「升级没生效」。

**来源要对得上**。如果当前生效的那份不是用 npm 装的，界面会提示：

> 当前生效路径来自 {来源}，请使用该来源的更新方式；VaneHub 不会用 npm 新增另一份副本来冒充升级。

这是有意为之——用 npm 再装一份只会让冲突更严重。

## 可用的操作

CLI 工具页按状态提供不同操作：**安装**、**升级**、**降级**、**已是当前版本**、**不可用**、**手动处理**。

「手动处理」意味着 VaneHub AI 判断这个状态它不该自动动手：

> 当前安装状态需要手动处理。请先刷新检测并查看安装诊断，再选择对应来源的修复方式。

## 认证

**认证不在 VaneHub AI 里做**。五个 CLI 各自管理自己的 Provider 凭据，存在它们各自的位置。

如果 Agent 在会话中提示要登录，去对应的 CLI 里完成认证，然后回到 VaneHub AI 刷新检测。

## Web 预览

**状态：仅 Web/mock。** 浏览器预览展示确定性的可用性与执行 fixture，**不会检测也不会认证本地 CLI**。看到「已安装」不代表你机器上真的装了。

判断依据见 [Runtime 与功能状态标签](runtime-labels.md)。

## 下一步

CLI 跑通后，去[创建第一个会话](first-session.md)。

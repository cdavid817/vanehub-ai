# 安装并认证 CLI

VaneHub AI **驱动你已经装好的 CLI**。**各家的订阅登录（OAuth）始终由 CLI 自己完成**，VaneHub AI 不代劳、也不会要求你输入订阅账号密码。

但**「配第三方大模型」是另一回事**——想让某个 CLI 去调 DeepSeek、OpenRouter 这类兼容端点时，可以在**设置 → Agent 配置**里配好并应用，不必手改配置文件。两者的分工见[快速开始 → 认证 / 配置模型](quick-start.md#15-认证--配置模型)。

不想装 CLI 也能开始：OnePiece 是内置的原生 API Agent，不需要任何 CLI，见[原生 API Agent](native-agent.md)。

## 前置条件

必须具备：

- 至少一个受支持的 CLI，以及对应的订阅或 API 凭据

按需具备：

- **Node.js 与 npm**——仅当选择 npm 安装源时需要（各 CLI 要求的最低 Node.js 版本见下文各自小节）；用官方原生安装器装的 CLI 不依赖 Node.js
- **Git**——使用变更视图、代码评审或 [Git Worktree](worktree.md) 时需要
- **SSH**——连接[远端工作区](remote-workspaces.md)时需要

## 两种安装方式

装 CLI 有两条路，**结果等价，差别在谁来跑这条命令**。

### 方式 A：在 VaneHub AI 里装

打开**设置 → CLI 管理**，每个 CLI 卡片只给出它的来源真正支持的操作——安装、升级、降级，或者什么都不做。选一个版本，复核 VaneHub 展示的计划，再确认；执行结束后自动刷新检测，并告诉你这次改动是否已经验证。

适合：本机已有 Node.js 22+，且你不介意这份 CLI 来自 npm，或者在 Windows 上来自 WinGet。

**两个前提要清楚**：

- **来源决定能力**。VaneHub AI 能驱动 npm、Windows 上的 WinGet，以及逐个 CLI 审核过的官方安装器；Homebrew、Bun、Volta、桌面应用自带与系统包只检测不改动。它绝不会把脚本管道喂给 shell，也绝不会在别人装好的那份旁边再装一份冒充升级。
- **Antigravity CLI 没有 npm 包**，它只有官方安装器这一个来源，而官方安装器不支持钉精确版本，因此界面给的是“升级到最新”而不是一个版本列表。

### 方式 B：在终端里用命令装

按各 CLI 官方说明执行安装命令，具体命令见下一节。装完回到**设置 → CLI 管理**点**刷新检测**。

适合：想用官方推荐的原生二进制（不依赖 Node.js）、需要 Homebrew/scoop 等特定来源、或者该 CLI 压根没有 npm 包。

> **别把两条路混着走**。同一个 CLI 既用 npm 装一份、又用安装脚本装一份，就会触发[安装冲突](#安装冲突)——届时 `PATH` 顺序决定实际跑起来的是哪一份，而升级往往作用在另一份上。

无论走哪条路，**认证都得在终端里完成**，见[先在终端里跑通](#先在终端里跑通)。

## 五个 CLI

VaneHub AI 支持五个外部 CLI Agent。装一个就能开始，不必五个都装。下表汇总各 CLI 的安装方式,各小节给出具体命令。

| Agent | 提供方 | 命令 | 依赖 | 推荐安装方式 |
| --- | --- | --- | --- | --- |
| Claude Code | Anthropic | `claude` | 无(原生二进制);npm 需 Node.js 22+ | 原生安装脚本 |
| Codex CLI | OpenAI | `codex` | 无(原生二进制);npm 需 Node.js 18+ | 一键安装脚本 |
| Gemini CLI | Google | `gemini` | Node.js 18+ | npm 全局安装 |
| OpenCode | sst(开源) | `opencode` | 无(原生二进制);npm 需 Node.js | 一键安装脚本 |
| Antigravity CLI | Google | `agy` | 无(Go 单文件二进制) | 一键安装脚本 |

### Claude Code

Anthropic 官方的命令行 AI 编程助手，VaneHub AI 里模型族归为 Anthropic。需要 Claude Pro / Max / Team / Enterprise 账号或 Anthropic Console API 额度。官方现推荐**原生二进制安装**(不依赖 Node.js;npm 包实际也是下载同一份原生二进制):

```bash
# macOS / Linux
curl -fsSL https://claude.ai/install.sh | bash
# Windows(PowerShell)
irm https://claude.ai/install.ps1 | iex
# npm(仍可用,需 Node.js 22+;切勿用 sudo npm install -g,改用 nvm 或调整 npm 全局前缀)
npm install -g @anthropic-ai/claude-code
```

认证在终端运行 `claude` 后按提示完成(首次会打开浏览器登录)。VaneHub AI 通过 `claude-sdk` 或 PATH 上的 `claude` 判定可用性,**不保存你的凭据**。验证:`claude --version`、`claude doctor`。

### Codex CLI

OpenAI 官方 CLI,模型族归为 OpenAI。需要 OpenAI 账号(Plus / Pro / Business / Edu / Enterprise 计划或 API Key):

```bash
# macOS / Linux
curl -fsSL https://chatgpt.com/codex/install.sh | sh
# Windows(PowerShell)
irm https://chatgpt.com/codex/install.ps1 | iex
# npm(需 Node.js 18+)
npm install -g @openai/codex
# Homebrew(macOS)
brew install --cask codex
```

认证在终端运行 `codex` 后选 "Sign in with ChatGPT" 完成。安装脚本默认从 `releases.openai.com` 下载,失败时回退 GitHub Releases(可用 `CODEX_INSTALLER_USE_RELEASES_OPENAI_COM=false` 强制走 GitHub)。验证:`codex --version`。

### Gemini CLI

Google 官方 CLI,模型族归为 Google。用 Google 账号认证(OAuth):

```bash
npm install -g @google/gemini-cli
```

认证在终端运行 `gemini` 后选 "Login with Google" 完成。免费个人账号额度约每分钟 60 次、每天 1000 次请求。

> **Gemini CLI 的消费级路径正在收缩**:Google 已宣布自 2026-06-18 起,Gemini Code Assist Individuals 及 Google AI Pro/Ultra 等消费级账号不再经 Gemini CLI 提供请求服务,其「Login with Google」路径不再可用;官方建议这些用户迁移到 [Antigravity CLI](#antigravity-cli)。Gemini Code Assist Standard 与 Enterprise 不受影响。API Key 与 Vertex 属于不同认证路径,是否受影响以 Google 官方说明为准。

### OpenCode

开源 CLI(`sst/opencode`),支持多家 provider,模型族在 VaneHub AI 内归为 Unknown。注意 GitHub 上同名的 `opencode-ai/opencode`(Go/Bubble Tea)是另一个不相关项目,VaneHub AI 对接的是 `sst/opencode`:

```bash
# macOS / Linux(一键脚本)
curl -fsSL https://opencode.ai/install | bash
# npm / bun / pnpm / yarn
npm i -g opencode-ai@latest
# Homebrew
brew install sst/tap/opencode
# Windows
scoop bucket add extras && scoop install extras/opencode
```

认证方式随你选择的 provider 而定,在终端运行 `opencode` 后配置。注意:OpenCode 不支持长上下文,VaneHub AI 会据此调整其上下文能力。

### Antigravity CLI

Google 官方 CLI(Gemini CLI 的继任者),命令是 `agy`(不是 `antigravity`),模型族归为 Google。**没有 npm 包**,只能通过官方安装脚本安装:

```bash
# macOS / Linux
curl -fsSL https://antigravity.google/cli/install.sh | bash
# Windows(PowerShell)
irm https://antigravity.google/cli/install.ps1 | iex
# Windows(cmd)
curl -fsSL https://antigravity.google/cli/install.cmd -o install.cmd && install.cmd && del install.cmd
```

二进制默认放在 `~/.local/bin`(macOS/Linux)或 `%LOCALAPPDATA%\Antigravity\`(Windows)。它没有 npm 包,CLI 管理页提供的是经审核的**官方安装器**动作(仅支持升级到最新版)。默认认证走 **Google 登录**并把凭据存进**系统钥匙串**;CLI 官方另支持 API Key 与兼容端点,但 **VaneHub 的 Agent 配置面板当前未纳管这些字段**,需要时按 Antigravity 官方方式在 CLI 自身环境中配置。若本机曾装过 Gemini CLI(存在 `~/.gemini` 目录),`agy` 首次运行会提示是否导入旧设置(MCP 配置、命令白名单、快捷键、主题);与 Gemini CLI 的 npm 安装互不冲突,可同时保留。

> **订阅登录一律由各 CLI 自管**。VaneHub AI 只检测「这个命令能不能跑起来」，不替你走完 OAuth 登录，也不保存由此产生的会话凭据。（你在**设置 → Agent 配置**里主动填写的第三方 API Key 是另一回事，那份由 VaneHub AI 存进系统凭据服务。）安装后建议跑一遍 `claude --version` / `codex --version` / `gemini --version` / `opencode --version` / `agy --version`,确认版本号正常输出后再在 VaneHub AI 中添加会话。

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

**官方订阅登录不在 VaneHub AI 里做**。五个 CLI 各自管理自己的订阅凭据，存在它们各自的位置。

如果 Agent 在会话中提示要登录，去对应的 CLI 里完成认证，然后回到 VaneHub AI 刷新检测。

**要换成第三方大模型则相反**——去**设置 → Agent 配置**建一份配置并应用即可，不必手改 CLI 的配置文件。见[工具与扩展 → Agent 配置](agent-configuration.md#agent-配置)。

## CLI 启动参数

五个 CLI 各自的命令行参数与 VaneHub AI 里的启动参数配置，统一收在[工具与扩展 → CLI 参数](agent-configuration.md#cli-参数)。OnePiece 没有 CLI，也就没有启动参数，它的等价配置在[Agent 配置](agent-configuration.md#agent-配置)。

## 下一步

CLI 跑通后，去[创建第一个会话](first-session.md)。

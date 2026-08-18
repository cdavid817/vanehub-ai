# 安装并认证 CLI

VaneHub AI **驱动你已经装好的 CLI**，不替你装模型、不代管 Provider 凭据。认证始终由各 CLI 自己完成，VaneHub AI 不会要求你输入 Provider 密码。

唯一的例外是 OnePiece——它是内置的原生 API Agent，不需要任何 CLI，API Key 由 VaneHub AI 保存。想跳过 CLI 直接开始，见[原生 API Agent](native-agent.md)。

## 前置条件

- Node.js 22+ 与 npm
- 至少一个受支持的 CLI，以及对应的订阅或 API 凭据

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

> **Gemini CLI 面向个人用户逐步停用**:Google 已宣布将 Gemini CLI 迁移到 Antigravity CLI,自 2026-06-18 起面向个人/免费用户(免费 / Pro / Ultra)逐步停用 Gemini CLI 及 Gemini Code Assist;官方建议迁移到 [Antigravity CLI](#antigravity-cli)。企业版 Gemini Code Assist Standard/Enterprise 及付费 API Key 渠道不受影响。

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

二进制默认放在 `~/.local/bin`(macOS/Linux)或 `%LOCALAPPDATA%\Antigravity\`(Windows)。因此 CLI 管理页对它不提供 npm 安装/升级/降级操作。它走 **Google 登录**并把凭据存进**系统钥匙串**,配置档里根本没有密钥字段。若本机曾装过 Gemini CLI(存在 `~/.gemini` 目录),`agy` 首次运行会提示是否导入旧设置(MCP 配置、命令白名单、快捷键、主题);与 Gemini CLI 的 npm 安装互不冲突,可同时保留。

> **凭据一律由各 CLI 自管**。VaneHub AI 只检测「这个命令能不能跑起来」，不替你走完登录，也不保存任何外部 CLI 的 Provider 凭据。安装后建议跑一遍 `claude --version` / `codex --version` / `gemini --version` / `opencode --version` / `agy --version`,确认版本号正常输出后再在 VaneHub AI 中添加会话。

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

## 各 CLI 常见参数参考

五个外部 CLI 各自有命令行参数,供在 VaneHub AI 中排查启动参数、脚本化调用时参考。各 CLI 更新较快,`--help` 常滞后于实际支持,完整清单以对应官方 CLI Reference 为准。

| 功能 | Claude Code | OpenCode | Codex CLI | Gemini CLI | Antigravity CLI |
| --- | --- | --- | --- | --- | --- |
| 非交互/单次执行 | `-p, --print` | `run "<prompt>"` | `exec "<prompt>"` | `-p, --prompt` | 无独立子命令,交互式为主 |
| 指定模型 | `--model` | `-m, --model provider/model` | `-m, --model`/`--profile` | `-m, --model` | 无需指定,自动路由 |
| 继续最近会话 | `-c, --continue` | `-c, --continue` | `resume --last` | `-r "latest"` | `-c` |
| 按 ID 恢复会话 | `-r, --resume` | `-s, --session <id>` | `resume <id>` | `-r "<id>"` | `--conversation <id>` |
| 跳过权限确认(高风险) | `--dangerously-skip-permissions` | `--dangerously-skip-permissions` | `--dangerously-bypass-approvals-and-sandbox` | `--yolo`/`--approval-mode yolo` | `--dangerously-skip-permissions` |
| 沙箱/权限模式 | `--permission-mode` | agent 的 `permissions` 配置 | `--sandbox`, `--ask-for-approval` | `--sandbox`, `--approval-mode` | 内置审批模式 |
| 输出格式(脚本用) | `--output-format json/stream-json` | `--format json` | `--json`, `--output-schema` | `-o, --output-format json` | —— |
| 附加工作目录 | `--add-dir` | `--dir` | `--cd` | `--include-directories` | —— |
| 版本/帮助 | `-v/--version`, `--help` | `-v/--version`, `-h/--help` | `codex --version` | `-v/--version`, `-h/--help` | `agy --version` |

各 CLI 高频参数:

- **Claude Code** —— `--model <alias|id>`(别名如 sonnet/opus/haiku)、`--permission-mode <default|acceptEdits|plan|bypassPermissions>`、`--allowedTools`/`--disallowedTools`、`--add-dir`、`--max-turns`/`--max-budget-usd`(仅 `-p`)、`--mcp-config`/`--strict-mcp-config`、`--worktree`/`--session-id`、`--verbose`。
- **OpenCode** —— `-m, --model <provider/model>`(固定格式如 `anthropic/claude-sonnet-4-6`)、`--fork`(从某会话分叉)、`--format json`、`--attach <server-url>`(连到已运行 `opencode serve`)、`--agent <name>`、`serve --port --hostname`(无 UI HTTP 后端)。
- **Codex CLI** —— `--profile <name>`(config.toml 预定义档)、`--sandbox <read-only|workspace-write|danger-full-access>`、`--ask-for-approval`、`--json`/`--output-schema`、`--ephemeral`(不落盘 rollout)、`--skip-git-repo-check`、`--image`(多模态)。
- **Gemini CLI** —— `-m, --model`(别名 auto/pro/flash/flash-lite)、`--sandbox`/`-s`、`--approval-mode <default|auto_edit|yolo|plan>`、`--checkpointing`(改文件前快照,可 `/restore` 回滚)、`--include-directories`、`--extensions`、`--worktree`。
- **Antigravity CLI** —— `agy -c`(继续上次)、`agy --conversation <id>`(恢复指定对话)、`agy --dangerously-skip-permissions`("Turbo 模式")。无需 `--model`(默认自动路由)。MCP/权限配置在 `~/.gemini/antigravity-cli/settings.json`。

> **权限参数是重点**:五款 CLI 都有"跳过确认/自动批准"类参数。VaneHub 的权限模板(只读/标准/信任/Yolo)决定是否附加这些高风险参数,**安全策略优先于便利性配置**——详见[权限审批](permissions.md)。

## 原生 Agent OnePiece 的"参数"

OnePiece 不走外部 CLI,没有上述命令行参数。它的等价配置是 **provider Profile**(在**设置 → Agent 配置**里管理):选 provider 目录条目、填 API Key(保存前校验)、发现并选定模型、按需配自定义兼容端点。Profile 的生命周期与凭据回滚详见[开发者指南:OnePiece native Agent](../../../developer-guide/zh-CN/src/onepiece-native-agent.md)。

## 下一步

CLI 跑通后，去[创建第一个会话](first-session.md)。

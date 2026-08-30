# 工具与扩展

## 功能概述

MCP 服务器、Prompt Hook、本地扩展、插件集成、SDK 依赖、CLI 管理与参数、Agent 配置都在设置中心集中配置，再按 Agent 下发，不必在每个 CLI 里各配一遍。

Skill 的管理见[管理 Skill](skill-management.md)。

## MCP 服务器

MCP 服务器把外部工具接给 Agent，在**设置 → MCP 服务器**中集中注册。三种传输方式、命名规则、连接测试与状态缓存、Claude Desktop 导入导出、中继范围、逐次工具审批与资源上限，见[MCP 服务器](mcp.md)。

## Prompt Hook

Prompt Hook 在提示词组装链路里插入内容，在**设置 → Prompt Hook** 中配置。七种分类、两个执行阶段、模板变量允许清单、草稿/发布/回滚与效果评估，见[Prompt Hook](prompt-hooks.md)。

> **Prompt Hook 只能绑定到五个外部 CLI Agent，不作用于 OnePiece**——原生 Agent 有自己的核心指令机制。

## 扩展能力

**设置 → 扩展能力**里装的是**本地多模态 AI 能力**，不是通用插件。首版每种能力提供一个内置白名单框架：

| 能力 | 框架 | 运行时 | 本地端口 | 预计磁盘占用 |
| --- | --- | --- | --- | --- |
| **OCR 文字识别** | PaddleOCR | Python 3.10+ | 9875 | **~1800 MB** |
| **语音识别** | faster-whisper | Python 3.10+ | 9876 | **~900 MB** |
| **语音合成** | sherpa-onnx | Python 3.10+ | — | — |

**装之前先看两件事**：需要本机有 Python 3.10+，以及**磁盘占用不小**——PaddleOCR 接近 1.8 GB。每个框架卡片上都有「安装要求」可展开查看。

页面顶部有**已安装 / 运行中 / 异常**三个计数，异常时到操作日志里查原因。

![设置中的扩展能力页面，PaddleOCR 与 faster-whisper 框架卡片](assets/screenshots/extensions-zh-CN.png)

## 插件集成

**设置 → 插件集成**管理内置产品集成与就绪检测——注意它**不安装第三方插件包**。首版内置 GitHub 一个集成，检测本机 `gh` 的认证状态。五种状态的含义、启用步骤与 Web 模式限制，见[插件集成](plugin-integration.md)。

## SDK 依赖

**受管 SDK 只有两个**：Claude Code SDK 与 Codex SDK，各自对应一个 npm 包，并带三个备选版本。

**没有独立的 SDK 设置页**。要换装哪个版本，在 [CLI 管理](#cli-管理)里操作——所有受管包的安装、升级、降级都走那一处，某个版本出问题时的回退也在那里。

Gemini CLI、OpenCode 与 Antigravity CLI 没有对应的受管 SDK。

## CLI 管理与参数

### CLI 管理

**设置 → CLI 管理**汇报这台机器上装了什么，并且对能驱动的来源代为改动。顶部状态条把每个工具计入且只计入一个桶——**就绪**、**待登录**、**可更新**、**有冲突**、**无法运行**——每个计数同时就是它的筛选按钮。搜索、来源筛选和"只看需要处理的"可以进一步收窄列表。

![设置中的 CLI 管理页面，五个 CLI 卡片与本地环境检查](assets/screenshots/cli-zh-CN.png)

#### 实际跑的那一份，和 VaneHub 会动的那一份

同一个 CLI 可能装了好几份，来自好几个来源。页面为每个工具报告两个身份，而不是一个：

- **PATH 命中**——你的 shell 真正会执行的那一份，只由 `PATH` 顺序决定。
- **推荐使用**——VaneHub 会操作的那一份，由探测时实际跑起来的结果决定。

两者只在出问题时才不一致，而这正是重点：`PATH` 中靠前的那份坏掉的启动器，意味着你在终端里看到的版本并不是页面报告为可用的那个版本。详情抽屉的**安装**分区列出每一份的完整路径、来源、来源可信度、`PATH` 位置，以及是否被遮蔽。

冲突是结构化的，不是一段自由文本。每条冲突都带类型、严重程度、涉及的安装，以及它是挡住变更、挡住启动，还是两者都挡。九种类型覆盖：重复启动器别名、`PATH` 遮蔽、坏条目优先、同时存在多个安装来源、版本分歧、来源归属不明、环境与 `PATH` 分歧、架构不匹配，以及启动器指向已不存在的目标。当某条冲突挡住变更时，VaneHub 会收起这个操作，而不是替你挑一份下手。

**VaneHub 不会修 `PATH`，不会删除重复安装，也不会把工具从一个来源迁到另一个来源。** 这三件事都是你没要求的机器改动，而且任何一件都可能弄坏 VaneHub 之外的东西。

#### 来源，以及每种来源能做什么

来源指这一份是从哪来的；在 VaneHub 能驱动它时，也指变更会怎么做。能力按来源、按动作区分，由后端给出，不靠名字猜：

| 来源 | VaneHub 能做 | 说明 |
| --- | --- | --- |
| **npm** | 按精确版本安装、升级、降级、重装、卸载 | 你选的版本就是装上的版本 |
| **WinGet** | 安装、升级、卸载 | 仅 Windows。降级与重装在各自单独验证之前保持关闭 |
| **官方安装器** | 安装、升级到最新 | 逐个 CLI 审核，仅 HTTPS，不支持钉精确版本 |
| **Homebrew、Bun、Volta、桌面应用自带、系统包、手动安装、未知** | 什么都不做——**仅检测** | 报告出来、解释清楚，然后不去碰它 |

**"仅检测"说的是 VaneHub，不是你的安装。** 一个由 Homebrew 装好、跑得很正常的 CLI，既是健康的，也是仅检测的；页面会说清楚真正拥有它的是哪个工具——"请用 `brew upgrade` 更新"——而不是给你一个没解释的缺失按钮。VaneHub 不会在旁边另装一份 npm 副本冒充升级。

**版本列表绝不在来源之间借用。** 一份 WinGet 安装的更新状态由 WinGet 自己的目录决定，绝不由 npm 的目录决定。

#### 执行之前先复核

选一个版本不会启动任何东西。VaneHub 先生成一份**操作计划**给你看：

- 动作、来源、通道
- 精确的版本变化，从哪个版本到哪个版本
- **将要执行的命令，以结构化参数列表呈现**——不是 shell 字符串，也不是把脚本管道喂给解释器
- 是否需要网络、是否需要提权
- 前置条件与注意事项
- 计划何时过期，以及一句明确的声明：失败**不会**悄悄改用别的来源

确认时提交的是这份计划的 ID 和你看到的那个版本号——只有这两样。这次调用上没有任何字段能重建出一条命令，这才让"你复核的版本就是实际执行的版本"成为设计上的性质，而不是一句承诺。

计划一次性使用，有效期十分钟。过期、已经执行过、或者环境在这期间变了，VaneHub 都会拒绝它，并提供重新准备一份。**选中你已经在用的版本时，根本不会给出任何操作**——没有可执行的东西。

#### 执行之后

包管理器是外部副作用。往数据库里写回一条旧记录并不能把它撤销，所以 VaneHub 只汇报它真正知道的：

| 结果 | 含义 |
| --- | --- |
| **已验证** | 命令成功，事后复检确认了新版本 |
| **已执行，未能验证** | 命令成功了；事后复检没能确认。请先刷新检测再依赖页面显示的版本——不要重复执行 |
| **已改动，但失败** | 命令失败，但复检显示这台机器仍然发生了改动。**没有回滚任何东西**，因为撤销一次外部安装不是 VaneHub 能做的事 |
| **失败，未改动** | 命令失败，且没有观察到任何改动，可以安全重试 |
| **已取消** | 你停掉了它。取消从不意味着已经生效的改动被撤销了 |

某个操作进行期间，只有它触及的那个工具处于忙碌状态。其他 CLI 依然可读、可操作，已缓存的信息也留在屏幕上——刷新期间自己清空的页面，在探测结束之前会一直被读成"什么都没装"。比当前环境更旧的数据会被标记为过期，而不是丢弃。

**全部升级**会先预览再执行，分两个列表：将执行的，和不执行的以及各自的原因——已是目标版本、来源仅支持检测、版本目录不可用、需要先登录、存在阻断性冲突等等。执行结束后，它知道的每个工具都带着上表中属于自己的那个结果。一项失败不会把其他项遮掉。

#### 诊断与登录

详情抽屉的**诊断**分区展示每个探测得出的结论：版本探测、工具自己的 doctor 命令、登录检查、兼容性。**`unknown` 就报成 `unknown`**——"这个 CLI 没有公开的非交互检查方式"和"检查失败了"是两件不同的事，把前者报成后者，正是让能正常工作的 CLI 看起来坏掉的原因。

**VaneHub 从不捕获任何服务商凭据。** 登录 Claude Code、Codex CLI 或其他任何 CLI，都发生在那个 CLI 里、经由那个厂商，凭据也留在那个厂商放它的地方。VaneHub 只运行有文档的状态命令，从输出里读出一个规范化的答案——已登录、需要登录、已过期、未知——除此之外什么都不存。原始探测输出在进入操作日志、本页面或日志文件之前，先被截断并脱敏。

### CLI 参数

**设置 → CLI 参数**配置各 CLI 的启动开关。

![设置中的 CLI 参数管理页面](assets/screenshots/settings-cli-parameters-zh-CN.png)

页面左侧是五个外部 CLI 的导航栏，每个条目下方显示该 CLI 的**已检测版本或安装状态**，以及未保存改动、警告、错误的计数。OnePiece 不在这里——它不经外部 CLI 启动，配置在**设置 → Agent 配置**。

**「继承」是一种独立状态，不是叫 `default` 的取值。** 保持继承时 VaneHub 不发送任何参数，由 CLI 自己决定；只有你显式选了值，它才会出现在启动命令里。这个区分是必要的：Gemini CLI 的 `--approval-mode default` 里，`default` 是「每次询问」这一真实模式，不是「没设置」。

参数带这些标注：

- **风险标注**——危险开关会被显著标记
- **启动场景**——顶部可切换「对话」与「交互式」。同一个 CLI 在两种场景下需要的参数不同，页面只列当前场景下真正生效的那些
- **成熟度与兼容性**——预览/实验/弃用状态，以及「已安装版本不支持此值」这类判定
- **依赖与冲突**——例如 OpenCode 的 `--variant` 依赖先设置 `--model`，未满足时会明确提示

**筛选与搜索**：可按「全部 / 已修改 / 警告 / 不受支持 / 高级」筛选，搜索框同时匹配名称、说明、选项文案和字面参数标志。

**预览是逐 token 的，按「全局选项」和「调用选项」分段。** 它刻意不拼成一行可粘贴的命令：含空格的取值在 argv 里是一个 token，被 shell 拆开后就是两个，拼接展示会误导。需要精确内容时用「复制 argv JSON」。

**保存与并发**：页面记住你打开时的版本号。若同一配置在别处被改动，保存会被拒绝并提示重新加载，而不是静默覆盖对方的改动。「放弃草稿」回到上次保存的状态，「恢复为继承」把该 CLI 的全部参数清回继承。切换 CLI 不会丢失草稿。

**旧数据修复**：从旧版本升级时，无法无歧义解读的历史取值会被隔离——它不会被发送，也不会被删除，页面上给出提示，重新选一次即可修复。

**改动何时生效**：参数在**下一次启动**时读取。已经在运行的对话或终端不受影响，保存不会打断它们。

> **权限模板会压过你在这里保存的选择**。例如当前是「只读」模板时，即使参数里勾了某个宽松选项，也以模板为准。安全策略优先于便利性配置。审批、自动批准、沙箱与危险绕过类参数不在本页——它们归[权限审批](permissions.md)管。

#### 各 CLI 常见参数参考

五个外部 CLI 各自有命令行参数，供在 VaneHub AI 中排查启动参数、脚本化调用时参考。各 CLI 更新较快，`--help` 常滞后于实际支持，完整清单以对应官方 CLI Reference 为准。

| 功能 | Claude Code | OpenCode | Codex CLI | Gemini CLI | Antigravity CLI |
| --- | --- | --- | --- | --- | --- |
| 非交互/单次执行 | `-p, --print` | `run "<prompt>"` | `exec "<prompt>"` | `-p, --prompt` | 无独立子命令，交互式为主 |
| 指定模型 | `--model` | `-m, --model provider/model` | `-m, --model`/`--profile` | `-m, --model` | 无需指定，自动路由 |
| 继续最近会话 | `-c, --continue` | `-c, --continue` | `resume --last` | `-r "latest"` | `-c` |
| 按 ID 恢复会话 | `-r, --resume` | `-s, --session <id>` | `resume <id>` | `-r "<id>"` | `--conversation <id>` |
| 跳过权限确认（高风险） | `--dangerously-skip-permissions` | `--dangerously-skip-permissions` | `--dangerously-bypass-approvals-and-sandbox` | `--yolo`/`--approval-mode yolo` | `--dangerously-skip-permissions` |
| 沙箱/权限模式 | `--permission-mode` | agent 的 `permissions` 配置 | `--sandbox`, `--ask-for-approval` | `--sandbox`, `--approval-mode` | 内置审批模式 |
| 输出格式（脚本用） | `--output-format json/stream-json` | `--format json` | `--json`, `--output-schema` | `-o, --output-format json` | —— |
| 附加工作目录 | `--add-dir` | `--dir` | `--cd` | `--include-directories` | —— |
| 版本/帮助 | `-v/--version`, `--help` | `-v/--version`, `-h/--help` | `codex --version` | `-v/--version`, `-h/--help` | `agy --version` |

各 CLI 高频参数：

- **Claude Code** —— `--model <alias|id>`（别名如 sonnet/opus/haiku）、`--permission-mode <default|acceptEdits|plan|bypassPermissions>`、`--allowedTools`/`--disallowedTools`、`--add-dir`、`--max-turns`/`--max-budget-usd`（仅 `-p`）、`--mcp-config`/`--strict-mcp-config`、`--worktree`/`--session-id`、`--verbose`。
- **OpenCode** —— `-m, --model <provider/model>`（固定格式如 `anthropic/claude-sonnet-4-6`）、`--fork`（从某会话分叉）、`--format json`、`--attach <server-url>`（连到已运行 `opencode serve`）、`--agent <name>`、`serve --port --hostname`（无 UI HTTP 后端）。
- **Codex CLI** —— `--profile <name>`（config.toml 预定义档）、`--sandbox <read-only|workspace-write|danger-full-access>`、`--ask-for-approval`、`--json`/`--output-schema`、`--ephemeral`（不落盘 rollout）、`--skip-git-repo-check`、`--image`（多模态）。
- **Gemini CLI** —— `-m, --model`（别名 auto/pro/flash/flash-lite）、`--sandbox`/`-s`、`--approval-mode <default|auto_edit|yolo|plan>`、`--checkpointing`（改文件前快照，可 `/restore` 回滚）、`--include-directories`、`--extensions`、`--worktree`。
- **Antigravity CLI** —— `agy -c`（继续上次）、`agy --conversation <id>`（恢复指定对话）、`agy --dangerously-skip-permissions`（"Turbo 模式"）。无需 `--model`（默认自动路由）。MCP/权限配置在 `~/.gemini/antigravity-cli/settings.json`。

> **权限参数是重点**：五款 CLI 都有"跳过确认/自动批准"类参数。VaneHub 的权限模板（只读/标准/信任/Yolo）决定是否附加这些高风险参数，**安全策略优先于便利性配置**——详见[权限审批](permissions.md)。

上表只列高频项。**由注册表生成、随代码更新的完整矩阵**——每个参数的字面标志、参数槽、启动场景、控件类型、归属、最低版本与核验状态——见[CLI 参数矩阵](../../../agent-infrastructure/cli-parameter-matrix.md)。**逐个参数族的完全参考**——调用形态、会话管理、模型选择、权限与沙箱、输出格式、配置注入，以及宿主按统一任务模型向各 CLI 投影参数的映射矩阵——见[AI 编码 CLI 参数完全参考](../../../agent-infrastructure/builtin-cli-reference.md)。

#### OnePiece 的等价配置

OnePiece 不走外部 CLI，没有上述命令行参数，因此它不是 CLI 参数页上的一个标签页。它真正有的配置全在**设置 → Agent 配置**里：**provider 配置**（选 provider 目录条目、填 API Key（保存前校验）、发现并选定模型、按需配自定义兼容端点），以及排在其下方的 OnePiece 检索、上下文压缩与上下文健康参数。见下一节与[原生 API Agent](native-agent.md)。

## Agent 配置

**设置 → Agent 配置**做的是一件和上面几节都不同的事：**决定各个 Agent 去调哪个厂商、哪个模型**。它是本页唯一会主动改写各 CLI 自己配置文件的功能。

![设置中的 Agent 配置页面，六个 Agent 标签与全局配置状态](assets/screenshots/settings-agent-configurations-zh-CN.png)

页面顶部按 Agent 分标签：**Claude Code / Codex CLI / OpenCode / Antigravity CLI / Gemini CLI / OnePiece**。同一页面下方还有[LSP 代码智能](lsp-code-intelligence.md)的语言服务器开关。

### 它解决什么

外部 CLI 的官方订阅登录（OAuth）VaneHub AI 管不了，那必须在终端里完成。但**换成第三方兼容端点**——DeepSeek、OpenRouter、智谱 GLM 之类——原本要你手改 `~/.claude/settings.json` 或 `~/.codex/config.toml`，现在在这个页面配好、应用即可。

内置目录含 **25 家 provider**（Anthropic、OpenAI 官方，以及 OpenRouter、DeepSeek、智谱 GLM、Kimi、Moonshot、SiliconFlow、阿里百炼、火山方舟、Groq、xAI、Mistral、Together、Fireworks、NVIDIA NIM、Cerebras、MiniMax、StepFun、百川、PPIO、七牛、ModelScope、小米 MiMo、Z.AI 等），也可以填自定义兼容端点。

### 各 CLI 能配到什么程度

| Agent | 第三方端点 | 纳管的配置文件 | 可配字段 |
| --- | --- | --- | --- |
| **Claude Code** | 支持 | `~/.claude/settings.json` | 端点、认证方式、主模型与 haiku/sonnet/opus 三档映射 |
| **Codex CLI** | 支持 | `~/.codex/config.toml`（`auth.json` 另需确认） | provider id、端点、模型、协议（Responses/Chat）、推理强度 |
| **OpenCode** | 支持 | `~/.config/opencode/opencode.json` | provider 定义、端点、npm 适配包、模型列表与默认模型 |
| **Gemini CLI** | 端点可改，但目录里只有 Google 官方预设 | `~/.gemini/.env` | 端点、模型、认证方式 |
| **Antigravity CLI** | **不支持** | `~/.gemini/antigravity-cli/settings.json` | 模型、工具审批模式、输出详细度、终端沙箱 |

> **Antigravity CLI 不接受自定义端点**。它只走 Google 登录、凭据存系统钥匙串，配置面板里没有端点和密钥字段——能调的是模型与审批行为。

Claude Code 与 Codex 是**互斥模式**：可以存很多份配置，但同一时刻只有一份处于「已应用」。OpenCode 是**累加模式**：provider 定义叠加保留，切换的只是全局默认的 `provider/model`。

### 应用时到底改了什么

- **只替换 VaneHub 拥有的那几个字段**。`~/.claude/settings.json` 里的 hooks、permissions、plugins 原样保留；`config.toml` 里的 projects、MCP server、注释和无关 provider 也不动。
- **先在内存里校验并构建完整结果，再原子替换**。Codex 涉及多文件时，任一步失败会把已改的文件全部还原。
- **切换配置前先回填**。离开某份配置时，VaneHub 会把当前生效文件里的纳管字段读回来写进那份配置，避免你在文件里的手工微调被静默丢弃。
- **应用后运行中的 CLI 进程不会自动重启**。VaneHub 不声称热重载，需要你自己重开会话或终端。

### 凭据与漂移

**API Key 存在操作系统的凭据服务里**，按 Agent/配置分账保存，不落 SQLite，界面上只回显「已配置」。只有在你显式点「应用」时，才会把明文写进那个 CLI 要求明文的配置文件。

应用成功后 VaneHub 会给纳管片段留指纹。**文件被外部改动时只报告漂移，不自动覆盖**；应用过程中若检测到文件正被并发改写，会中止写入而不是硬写。OpenCode 的外部编辑要等下次启动或手动导入才会并入。

启动时会做一次同步：Claude Code 与 Codex 在完全没有配置时，从可解析的现有文件导入一份 `default`（不回写文件）；一旦已有配置，后续启动就跳过。

### OnePiece

原生 Agent OnePiece 在同一个页面配置，但它的 Key 由 VaneHub AI 直接保管，且**保存前会实际调用一次校验，不通过不保存**，校验通过后再拉取可用模型列表。见[原生 API Agent](native-agent.md)。

## 注意事项与限制

- **全部仅桌面端可用**。
- **漂移只报告不自动修复**——检测到配置被外部改动时需要你确认处理方式。
- **MCP、Prompt Hook、扩展能力、CLI 参数都不改写各 CLI 自己的配置文件**，绑定通过启动参数与中继实现。**只有 Agent 配置例外**，它按上面的语义显式改写纳管字段。

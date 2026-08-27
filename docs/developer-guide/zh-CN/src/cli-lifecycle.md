# CLI 生命周期与全局配置

`tooling` 是最大的限界上下文，其中 Skill 与 MCP 两个子域各有专章（[Skill 管理](skill-management.md)、[MCP 工具与客户端](mcp-tools.md)）。本章讲另一半：**发现 CLI 本身、解决冲突、生成并执行变更计划，以及把 provider 配置写进各 CLI 自己的配置文件**。

## 目录是编译期常量

`CLI_TOOL_DEFINITIONS` 是 `&[CliToolDefinition]` 常量切片，不是运行期可扩展的注册表。每一项声明要查找的可执行文件名、这个 CLI 有哪些分发来源，以及可以对它跑哪些探测：

| Agent | 可执行文件 | 分发来源 |
| --- | --- | --- |
| Claude Code | `claude` | npm `@anthropic-ai/claude-code`、WinGet `Anthropic.ClaudeCode`、官方安装器 |
| Codex CLI | `codex` | npm `@openai/codex` |
| Gemini CLI | `gemini` | npm `@google/gemini-cli` |
| OpenCode | `opencode` | npm `opencode-ai`、官方安装器 |
| Antigravity CLI | `agy` | 只有官方安装器 |

分发来源自带能力声明，所以「这个能不能降级」是定义上的数据，而不是散落在界面里的某个条件判断。npm 支持按精确版本执行全部动作；WinGet 支持安装、升级、卸载，降级与重装在各自单独验证之前刻意关闭；官方安装器只能装到最新，不钉版本。

## 两个身份，而不是一个「生效安装」

发现流程按真实顺序走一遍 `PATH`，再枚举一组有界的已知位置，绝不递归扫盘。它产出的是一组安装，而快照会点名其中两个：

- `path_selected_installation_id`——shell 真正会执行的那份，只由 `PATH` 顺序决定。
- `recommended_installation_id`——后端会操作的那份，由探测结果决定。

它们是两个字段，因为它们回答的是两个问题。把两者合成一个，正是 `PATH` 中靠前的坏启动器能藏起来的原因：页面报告的是健康那份，终端跑的是坏那份。

启动路径遵循同一套划分。`CliApi::resolve_executable` 读这份快照、跟随推荐安装，返回的要么是**绝对路径要么是空**——裸命令名会在子进程里重新走一遍 `PATH` 解析，而 `PATH` 恰恰是争议本身。

## 冲突是结构化的值

`derive_conflicts` 产出零到多个 `CliConflict`，每条都带类型、严重程度、涉及的安装、`blocks_mutation`、`blocks_launch`，以及一个由前端本地化的稳定 `reason_code`。共九种类型：

`duplicate-launcher-alias`、`path-shadowing`、`broken-path-precedence`、`multiple-installation-sources`、`version-divergence`、`ambiguous-source-ownership`、`environment-path-divergence`、`architecture-mismatch`、`stale-launcher-target`。

比这份清单更重要的是两条性质：

- **`blocks_mutation` 与 `blocks_launch` 由后端判定**。界面若从类型自行推导，只要某个类型的严重程度变一次，两边就会各说各话。
- **先折叠启动器家族**。Windows 上一次 npm 全局安装会并排写下 `tool`、`tool.cmd`、`tool.ps1`；不折叠的话，一份安装会被报告成三份互相竞争的安装。

## 来源决定能力，能力绝不从名字猜

`CliSourceKind` 说明这一份是从哪来的：`Npm`、`Winget`、`VendorInstaller`、`Homebrew`、`Bun`、`Volta`、`Desktop`、`System`、`Manual`、`Unknown`。`CliSourceManagement` 说明 VaneHub 能拿它怎么办——前三个是 `managed`，其余是 `detect-only`。

**「仅检测」说的是 VaneHub 的能力，绝不是这份安装的健康状况。** 一个由 Homebrew 装好、跑得正常的 CLI，同时是健康的和仅检测的。每种仅检测的来源都带一个 `guidance_code`，点名真正拥有它的工具，于是「为什么没有升级按钮」的答案是「请执行 `brew upgrade`」，而不是「不支持」。

版本目录按来源区分。一份 WinGet 安装的更新状态来自 WinGet 自己的目录；借用 npm 的目录正是这套模型要消除的缺陷。

`CliSourceConfidence` 把 `unknown`、`inferred`、`verified` 分开。路径启发式得出的是 *inferred*——足以提供一个操作，不足以宣称归属。

## 操作计划就是契约

没有计划就不会有任何改动。`prepare_cli_action` 接收用户选的东西——agent、来源、目标版本、通道——返回一个操作 ID；它产出的计划带着后端推导出的动作、精确的版本变化、**结构化的 `argv` 预览**、前置条件、注意事项、提权与网络需求，以及一个过期时间。

`execute_cli_action` **只接收计划 ID 和用户看到的那个版本号**。这次调用上没有任何参数能重建出一条命令，这才让「复核的版本就是执行的版本」成为结构性质而非约定。计划一次性使用、有效期十分钟、绑定环境指纹；过期、复用、版本号不匹配、环境已变，是四种不同的拒绝，对应四个稳定的类别。

方向只在一个地方推导。`action: null` 表示「把这个工具挪到所选版本」，由后端判断这是安装、升级还是降级——整个产品里只有一条版本比较路径 `NormalizedCliVersion`，而解析不了的版本保持*不透明*，绝不靠猜。

**没有回退。** 官方安装器失败不会悄悄变成一次 npm 安装，而且每份计划都会把这一点写在脸上。

## 下载一个你马上要运行的程序

VaneHub 唯一「取回一个程序然后执行它」的路径已经不在 `tooling/cli` 里了。它住在 `tooling/managed_install`，`tooling/cli` 是它的第一个使用方而不是它的所有者。

搬走的是绝不能存在两份的那部分：HTTPS 加精确主机白名单，且对**每一跳重定向**都检查而不是只检查原始 URL；字节上限在**读的过程中**卡住，而不是写完再看长度；跳与跳之间、以及流式读取过程中都检查截止时间与取消；执行任何东西之前先做 SHA-256 校验；临时目录在成功、失败、超时、取消四条路径上一律释放。这里面任何一条复制一份都会漂移，而且漂移的方式在单独审查那份副本时**看不出来**——一个只检查首个 URL、不检查后续跳转的重定向循环，孤立地看完全正确。

留下的是真正属于 CLI 工具的那部分：安装器模板、它们的解释器与版本参数、`CliPlatform`，以及那个「精确匹配、没有兜底分支」的平台选择——缺少的那个兜底分支正是当年 Windows 上生成 `bash -lc` 计划那个缺陷的修复。`CliInstallerTrust` 现在把原来那三个约束字段换成了一个内嵌的 `RetrievalPolicy`。

只搬了这一半。发现、生效解析、冲突、版本目录、操作计划、变更协调、持久化和管理 UI 全都还在这里——把它们也抽出去意味着一次触及持久化主键的、24,000 行规模的行为保持重构，而用户侧一无所得。

`managed_install` 里还有一个有界的归档解压器，`tooling/cli` 一个字都没调用它。它存在是因为以归档形式分发的语言服务器需要它，而把它设计在下载边界旁边，正是为了不让下一个使用方再写一套边界出来。它的包含性检查跑在每个条目**解析后**的路径上：只扫描开头的分隔符会漏掉 `a/../../b`——它看着像个普通相对路径，规范化之后却跑出了目标目录。

## 外部副作用发生之后

包管理器没法靠写回一条旧记录撤销，所以结果词汇区分五种终态：`verified`、`applied-unverified`、`changed-but-failed`、`no-change-failed`、`cancelled`。

`changed-but-failed` 是让其余几种成立的那一个。当命令失败但事后检测观察到机器已经变了，诚实的汇报是「发生了一些事，而且不是被要求的那件事」。**绝不把操作前的快照恢复回去当成回滚**；而当检测本身失败时，上次已知的值会被保留、标记为过期，并附上警告。

## 全局配置：改写各 CLI 自己的文件

`cli_config` 子域是 `tooling` 里唯一**主动改写外部程序配置文件**的部分。五个 Agent 各有纳管文件，语义见 [CLI Agent 全局配置](../../../cli-agent-global-configuration.md)。

四条写入约束：

- **只替换 VaneHub 拥有的字段**。`settings.json` 里的 hooks、permissions、plugins，`config.toml` 里的 projects、MCP server、注释与无关 provider，全部原样保留。
- **先在内存里构建完整结果，再原子替换**。Codex 涉及多文件时，任一步失败会把已改的文件全部还原。
- **切换配置前先回填**。离开某份配置时，把当前生效文件里的纳管字段读回写进那份配置——否则你在文件里的手工微调会被静默丢弃。
- **漂移只报告不覆盖**。应用后给纳管片段留指纹；文件被外部改动时报告漂移，应用过程中检测到并发改写则中止写入。

凭据按 Agent/配置分账存在操作系统凭据服务里，不落 SQLite；只有显式「应用」时才把明文写进那个 CLI 要求明文的文件。**VaneHub 自己从不捕获任何服务商凭据**：登录属于厂商自己的 CLI，登录探测里读出来的只有一个规范化的结论。

**应用成功后运行中的 CLI 进程不会自动重启**——不声称热重载。

## 与旧模型的兼容

旧模型留下的 `cli_tool_status` 表仍然会被创建、仍然可读，好让升级上来的安装在第一次刷新之前也能看到自己的工具。它**只读，且永不权威**：只有在不存在真实快照时，遗留行才会被映射成一份*过期*快照；没有任何地方写它；一旦出现第二个读取方或任何写入方，架构测试就会让构建失败。

## 与其他上下文的关系

- 检测到的 CLI 如何成为可用 Agent，见 [Agent 生命周期与 provider 运行时](agent-lifecycle.md)。
- CLI 起进程之后的交互路径见[终端与 PTY 运行时](terminal-runtime.md)；非交互的委派路径见 [CLI 委派与 ChangeSet 管线](cli-delegation.md)。
- provider 目录与 OnePiece 共用同一份，见 [OnePiece native Agent](onepiece-native-agent.md)与[内置模型提供商目录](../../../model-providers.md)。
- 用户侧流程见用户指南的安装并认证 CLI 一章。

## 设计所在

本章用于为贡献者定向，权威需求位于 `openspec/specs` 下对应能力的主规范中。

# Provider 接入矩阵与证据边界

核验日期：2026-09-06。以下是目标适配契约与官方资料快照，不是已安装/已联调的本机版本清单。精确来源见 [official-sources.md](references/official-sources.md)。禁止凭表格将未知能力全部设为 true。

## 1. 身份、入口与来源

| 优先顺序 | 稳定 ID | CLI 与可执行名 | 官方 ACP 入口 | 首版建议管理来源 | 证据 |
| --- | --- | --- | --- | --- | --- |
| P0 | qwen-code | Qwen Code / qwen | `qwen --acp` | npm `@qwen-code/qwen-code`；其他来源独立审查 | S01–S03 |
| P0 | kimi-cli | Kimi Code CLI / kimi | `kimi acp` | npm `@moonshot-ai/kimi-code`；native 与旧 uv 分开检测 | S04–S07 |
| P1 国内 | qoder-cli | Qoder CLI / qoder | `qoder --acp` | npm `@qoder-ai/qodercli`；vendor 另行受审查 | S08–S09 |
| P1 国内 | codebuddy-code | CodeBuddy Code / codebuddy | `codebuddy --acp` | npm `@tencent-ai/codebuddy-code`；native 独立审查 | S10–S11 |
| P1 | copilot-cli | GitHub Copilot CLI / copilot | `copilot --acp --stdio` | npm `@github/copilot`，可按现有 WinGet adapter 扩展 | S12–S13 |
| P1 | cursor-agent-cli | Cursor Agent CLI / agent | `agent acp` | reviewed vendor 或 detect-only，不猜 npm 包 | S14–S15 |
| P2 Legacy | iflow-cli | iFlow CLI / iflow | 本变更不声明 | 本地 detect-only，不默认自动安装 | S16 |

P0/P1/P2 仅表示依赖顺序。所有行都在当前 change 范围；iFlow 的限制是明确的产品边界，不是遗漏。

## 2. 六种活跃 CLI 的目标闭环

六种均要求：安装身份与版本 → 来源/登录说明 → 原生终端 → ACP 统一对话 → 工具事件与适用审批 → 取消/故障 → 会话绑定 → 可选恢复/模型/usage 的真实结果 → 多 Agent/定时能力门控。原生终端不能成为绕过硬策略的路径。

“支持”有三层，必须区分：

| 层次 | 证据 | 不代表什么 |
| --- | --- | --- |
| upstream-documented | 官方文档有对应命令/协议 | 不代表本机 binary/账号支持 |
| contract-tested | 精确版本契约对应的静态/协议桩测试通过 | 不代表已运行真实模型或三平台 |
| live-verified | 授权环境中的真实 CLI、版本、平台和功能 smoke 有记录 | 不代表其他版本/架构也通过 |

适配器中可选能力包括 session/load、图片、MCP source/type、模型/配置切换、usage/费用与 reasoning。均基于握手、已实现支持和测试，不要求所有 CLI 功能完全一致。

## 3. 必须落成测试的各家差异

### Qwen

官方配置入口为 `--acp`；headless 的结构化输出和双向输入必须独立验证。禁止依据 Gemini 兼容性推断 Qwen。官方 standalone installer 有 npm fallback 描述，因此自动安装首选已披露的 npm 计划；未确认禁止内部切源前不把该 vendor 脚本接成“执行固定来源”的自动操作。不同时间页面对 Node 最低版本可能不同，使用所选发行包 engines 与实际 release 证据，不套用旧博客的最低版本。

### Kimi

官方新版本已有从 Python/uv 迁移的说明；npm、native 与旧 Python 的依赖各自判断，不能把“代码实现基于 Node”误写成所有二进制安装都必须装 Node。`kimi -p` 文档明确不进行人工审批，常规工具按 auto 策略处理且静态 deny 仍保留；因此不能称其完全无限制，也不能作为需要逐次人工审批的 fallback。配置与外部 session 绑定发行形态，迁移须用户另行批准。

### Qoder

通过 `qoder --acp`；使用自身登录状态或经批准的 profile。官方当前说明 Windows arm64 不支持，应显示精确限制而不是笼统“全平台”。`qodercli` 等历史命令仅在实际版本身份和参数明确匹配时保留别名，不默认复用旧 grammar。

### CodeBuddy

账号环境不是 UI 语言。官方环境规则为国际默认不设，国内 `internal`，iOA `ioa`；只在对应已验证 profile 注入。客户端声明 fs/terminal 会引发操作代理，因此不能先声明能力再补权限。内部 Team 事件归属同一 VaneHub seat；不借此实现新的平台多 Agent scheduler。

### Copilot

使用独立 `copilot`。某些 tools/reasoning 选项在 ACP server 启动时固定，并影响该进程创建/加载的会话，首版每 binding 独立进程，禁止跨策略复用。并非所有 BYOK 情形都需要相同登录流程，认证以选定 profile 与官方契约判定，不统一硬编码“必须 GitHub 登录”。

### Cursor

`agent` 必须身份校验。`cursor/ask_question`、`cursor/create_plan` 为阻塞请求，必须明确响应或取消；todo/task 等通知不回 RPC response。官方当前安装文档中的 Windows 原生 installer URL 为 `https://cursor.com/install?win32=true`，不要从其他厂商惯例猜 `install.ps1`。自动安装仍需下载到受限临时文件、审查与披露，不能照抄 pipe-to-shell。未验证的更名别名不自动接受。

### iFlow

官方服务与 API 在 2026-04-17 关闭，但已安装 CLI 可走用户配置的自定义 API。提供 legacy 本地终端兼容，不为它增加新官方账号登录、自动安装或虚假的 ACP 支持。不自动把 iFlow session/配置转换成 Qoder。

## 3a. 已按真实 `--help` 核验的启动旗标（2026-09-06，本机安装版本）

| CLI | 只读 | 标准 | 可信 | Yolo | 终端恢复 | ACP 入口 |
| --- | --- | --- | --- | --- | --- | --- |
| Qwen 0.23.0 | `--approval-mode plan` | `--approval-mode default` | `--approval-mode auto-edit` | `--approval-mode yolo` | `--resume <id>` / 新会话 `--session-id <uuid>` | `--acp` |
| Kimi 0.41.0 | `--plan` | 默认（询问） | `--yolo`（常规自动、高风险仍问） | `--auto`（从不询问） | `--session <id>` | `acp`（另有 `acp --login` 终端登录入口） |
| Qoder 1.1.45 | 无 plan 模式 → 终端拒绝 | `--permission-mode default` | `--permission-mode accept_edits` | `--permission-mode accept_edits` | `--resume <id>` / `--session-id <id>` | `--acp`（`--help` 未列出，握手已验证） |
| CodeBuddy 2.146.0 | `--permission-mode plan` | `--permission-mode default` | `--permission-mode acceptEdits` | `--permission-mode acceptEdits` | `--resume <id>` / `--session-id <uuid>` | `--acp`（`--acp-transport stdio` 默认） |
| Copilot 1.0.83 | `--mode plan` | 默认（询问） | `--allow-all-tools` | `--allow-all-tools` | `--resume=<id>` | `--acp --stdio` |
| Cursor 2026.09.02 | `--mode plan` | 默认（询问） | `--force` | `--force` | `--resume <id>` | `acp` |
| iFlow 0.5.19 | `--plan` | `--default` | `--autoEdit` | `--yolo` | `--resume <id>` | 不声明（程序有 `--experimental-acp`，本变更不接入） |

ACP 传输下只传入只读姿态的旗标（agent 逐次经 `session/request_permission` 询问宿主）；上表中的可信/Yolo 旗标仅用于原生终端。修正记录：Qwen 此前误写为 `auto_edit`（真实值 `auto-edit`）。

## 4. 待执行者填写的版本证据

| ID | 本机版本/发行形态 | 平台/架构 | 入口核验 | fake suite | 真实 smoke | 结论 |
| --- | --- | --- | --- | --- | --- | --- |
| qwen-code | 0.23.0，npm `@qwen-code/qwen-code`（`~/.npm-global/bin/qwen` → `cli-entry.js`） | Linux x86_64（Ubuntu 24.04） | `qwen --acp` 真实握手：protocol 1、loadSession=true、agentInfo `qwen-code 0.23.0`、authMethods `["openai"]` | PASSED | 身份/发现/握手 PASSED（3.6 s）；文本 / 工具审批 / 拒绝 / 取消 / 重启后恢复轮次全部 PASSED（2026-09-07，用户提供的 DeepSeek OpenAI 兼容端点，经 VaneHub ACP 适配器；直连 CLI 文本轮次 14 s 亦通过） | live-verified（Linux 全流程；Windows / macOS NOT RUN） |
| kimi-cli | 0.41.0，npm `@moonshot-ai/kimi-code`（`dist/main.mjs`） | 同上 | `kimi acp`：protocol 1、loadSession=true、`Kimi Code CLI 0.41.0`、authMethods `["login"]` | PASSED | 同上（2.3 s） | live-verified（仅握手层）；旧 uv 发行形态本机未安装，未验证 |
| qoder-cli | 1.1.45，npm `@qoder-ai/qodercli`（`qoder` 与别名 `qodercli` 均在 PATH，同一包） | 同上 | `qoder --acp`：protocol 1、loadSession=true、`qoder-cli 1.1.45`、authMethods `["qodercli-login"]` | PASSED | 同上（5.4 s） | live-verified（仅握手层） |
| codebuddy-code | 2.146.0，npm `@tencent-ai/codebuddy-code`（`bin/codebuddy`） | 同上 | `codebuddy --acp`：protocol 1、loadSession=true、agentInfo 未提供、authMethods `["iOA","external","internal","selfhosted"]`（账号环境同时作为 auth method 暴露） | PASSED | 同上（2.4 s，international profile） | live-verified（仅握手层）；china/ioa profile 的握手未单独执行 |
| copilot-cli | 1.0.83，npm `@github/copilot`（`npm-loader.js`） | 同上 | `copilot --acp --stdio`：protocol 1、loadSession=true、`Copilot 1.0.83`、authMethods `["copilot-login"]` | PASSED | 同上（1.3 s） | live-verified（仅握手层）；WinGet 来源 NOT RUN |
| cursor-agent-cli | 2026.09.02-c22c1a3，vendor installer（`~/.local/bin/agent` → `~/.local/share/cursor-agent/versions/2026.09.02-c22c1a3/cursor-agent`） | 同上 | `agent acp`：protocol 1、loadSession=true、agentInfo 未提供、authMethods `["cursor_login"]`；`--version` 输出不含 "cursor"，身份由规范路径标记 `cursor-agent` 接受 | PASSED | 同上（1.7 s）；`cursor/ask_question`/`create_plan` 真实触发 BLOCKED | live-verified（仅握手层） |
| iflow-cli | 0.5.19，npm `@iflow-ai/iflow-cli`（用户授权后手动安装，应用内仍为 detect-only） | 同上 | 发现与 `--version` PASSED；不声明 ACP | PASSED（拒绝路径） | 原生终端启动未执行；自定义 API 路径 PASSED（2026-09-07，用户提供的 DeepSeek 兼容端点，`selectedAuthType: openai-compatible` + `IFLOW_*` 环境变量，直连 CLI 28 s 返回 `OK`） | detect-only 已验证；自定义 API 直连已验证 |

未登录时 `session/new`：Qwen、Kimi、Qoder、CodeBuddy、Cursor 以 `-32000 Authentication required` 拒绝（本实现归类为 `acp-authentication-required`）；Copilot 接受 `session/new`（未发送 prompt）。

安装均由用户明确授权后在本机手动完成（npm 全局固定版本；Cursor 安装脚本先下载到本地审阅再执行）；未执行任何登录、未调用任何模型。证据：[verification/live-handshake-2026-09-06.log](verification/live-handshake-2026-09-06.log)。

实施时添加实际测试版本行，不把上表“文档已核验”修改成“全链路通过”。来源文档有变动时，先记录差异与新的 reviewed profile，然后更新规范与 fixtures。

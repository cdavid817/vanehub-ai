# Terminology

Words the documentation uses with one meaning. When a page and this list disagree, fix the page. A Simplified Chinese summary is at the end.

## Agents

- **Agent** — anything VaneHub AI can run a session with. Registered in the agent registry; identified by a stable id such as `claude-code`.
- **CLI Agent / external CLI agent** — an agent backed by a vendor's command-line program that you install and authenticate yourself.
- **OnePiece** — the built-in native API agent. It calls model providers over HTTP directly and needs no CLI.

## Transports

- **native API** — OnePiece's path: VaneHub calls the model provider itself and runs every tool through its own tool catalog.
- **headless** — VaneHub drives the CLI as a child process with a structured output format and reads its stream; used for the managed conversation of Claude Code, Codex CLI, OpenCode, Gemini CLI, and Antigravity CLI.
- **terminal PTY** — the Agent Terminal: the CLI's own interactive surface inside a pseudo-terminal. Every CLI agent has it; for a legacy agent it is the only surface.
- **ACP stdio** — the Agent Client Protocol over stdin/stdout: the CLI stays running for the session and asks the host per call for permissions, questions, and plans. Used by Qwen Code, Kimi Code CLI, Qoder CLI, CodeBuddy Code, GitHub Copilot CLI, and Cursor Agent CLI.

## Status vocabulary

| Word | Meaning | Evidence required |
| --- | --- | --- |
| **stable** (released) | In the current stable download | The release tag's manifest (`managedCliAgentIds` at that tag) or its release assets |
| **main** (unreleased) | Merged on `main`, not in any stable release yet | The commit on `main` |
| **fixture-qualified** | Automated suites pass with fixture CLIs; no recorded real-CLI run | The CI gate or desktop fixture layer |
| **live-qualified** | A real CLI, on a named operating system, at a named upstream version, on a named date | A record in the repository (OpenSpec tasks, `docs/desktop-release-verification.md`, evidence directory) |
| **experimental** | Shipped behind an explicit opt-in, semantics may change | The opt-in switch |
| **legacy** | Kept for detection or terminal use only; no managed conversation, no automation | The `legacy` capability tag |
| **planned** (spec-only) | Written in a proposal or issue, no code on `main` | The proposal |

Rule: a claim that something is **supported** or **delivered** must map to a main OpenSpec specification (`openspec/specs/`) or to a release manifest. An active change under `openspec/changes/` is evidence of intent, never of delivery, even when its code is on `main`; describe that state as **main / unreleased** and, where live tasks are still open, **fixture-qualified**.

## Supported, managed, verified, available, released

- **upstream supports** — the vendor CLI can do it by itself.
- **VaneHub manages** — VaneHub writes the setting or drives the flow (for example, writing a provider profile into a CLI's configuration file).
- **VaneHub verifies** — a recorded test proves the behaviour in VaneHub.
- **available** — the CLI is detected on this machine right now (an availability state, not a capability).
- **released** — see *stable* above.

"VaneHub does not manage X" is never the same sentence as "upstream does not support X". Antigravity CLI is the standing example: the upstream CLI supports API keys and compatible endpoints; VaneHub does not yet manage those fields.

## Configuration words

- **profile / provider configuration** — the model endpoint, key, and default model VaneHub writes into a CLI's own configuration, or stores for OnePiece. Modes: `exclusive`, `additive`, `apply-only`, `not-applicable`, `native-catalog` (defined in the [capability matrix](agents/capability-matrix.md)).
- **authentication / vendor login** — proving who you are to the vendor. Always happens in your terminal; VaneHub never brokers or stores subscription credentials.

## Permission words

- **policy template** — `readonly`, `standard`, `trusted`, `yolo`; assigned per agent principal.
- **host layer** — the unified decision point in VaneHub that resolves every gated action to Allow, Deny, or Ask. `trusted` and `yolo` are identical here; `yolo` only demands a confirmation to assign.
- **launch projection** — what the template becomes on the CLI's command line or environment. Differs by transport; identical for `trusted` and `yolo` on most CLIs, distinct on Qwen Code, Kimi Code CLI, and iFlow CLI terminals.
- **host-projected / provider-delegated / not enforceable** — the typed answer to "who guarantees this template on this launch": VaneHub passed and verified a flag; VaneHub passed nothing and the CLI's own prompt applies; or the launch is refused because no flag can express the template.

---

## 简体中文摘要

- **Agent / CLI Agent / OnePiece**：可建会话的对象 / 由厂商命令行程序支撑、你自行安装认证的 Agent / 内置原生 API Agent，无需 CLI。
- **传输方式**：原生 API（OnePiece）；headless（VaneHub 以结构化输出驱动 CLI 子进程）；终端 PTY（CLI 自己的交互界面）；ACP stdio（CLI 常驻并逐调用向宿主请求权限）。
- **状态词汇**：`stable`＝在当前稳定下载包中；`main`＝已合入 `main` 但未发布；`fixture-qualified`＝仅通过桩程序自动化测试；`live-qualified`＝在指定操作系统、指定上游版本、指定日期用真实 CLI 验证并有记录；`experimental`＝需显式开启；`legacy`＝仅检测或终端使用，无受管会话；`planned`＝仅有提案。"已支持/已交付"必须能对应到主规范或发布清单，不能只指向 active change。
- **上游支持 ≠ VaneHub 管理 ≠ VaneHub 已验证**；"VaneHub 不管理 X"不能写成"上游不支持 X"。
- **权限**：策略模板在宿主决策点统一执行；启动投影随传输方式不同；`trusted` 与 `yolo` 在宿主层等价，仅在 Qwen Code、Kimi Code CLI、iFlow CLI 的终端上投影不同。

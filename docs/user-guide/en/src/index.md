# VaneHub AI User Guide

<a href="../../zh-CN/src/index.md">简体中文</a>

This guide is written for **developers using VaneHub AI**: how to install it, how to use it, and what to check when something goes wrong.

## Five steps to get going

| Order | Chapter | What it covers |
| --- | --- | --- |
| 1 | [Quick Start](quick-start.md) | Install it, get it running, send the first message |
| 2 | [Install and authenticate a CLI](getting-started.md) | Installing and authenticating the five CLIs, and how to read availability |
| 3 | [Create your first session](first-session.md) | Choosing an Agent and a workspace, and the nine session workspace tabs |
| 4 | [Core concepts](core-concepts.md) | What session, seat, workspace, permission, Loop, and MCP each mean |
| 5 | [User interface](user-interface.md) | Everything the interface can do, feature by feature |

In a hurry, chapter 1 is enough; come back to the rest as needed.

## Feature deep-dives

| Chapter | What it covers |
| --- | --- |
| [Multi-Agent group chat](multi-agent-workflow.md) | Several Agents in one session, handing the turn over with `@` |
| [Group chat collaboration case](multi-agent-testing-tutorial.md) | Walking UI, handoff, and historical identity acceptance with an architect, an implementer, and code review |
| [Git worktrees](worktree.md) | Let an Agent edit code in its own working copy without touching your branch |
| [Loop Engineering](loop-engineering.md) | Set a goal and must-pass checks, and let it iterate until it gets there |
| [Goal management](goal-management.md) | Tracking plans, Loops, and work items under one objective |
| [Todo Board](todo-board.md) | Manual to-dos and Agent activity on one board |
| [Agent evaluation](evaluation.md) | Run several Agents head-to-head on the same task; compare pass rate, tokens, and time |
| [Slash commands](slash-commands.md) | Switching tabs, flipping switches, and checking usage from the input box |
| [Code review](code-review.md) | Reading the diff line by line, commenting, and sending feedback to the Agent |
| [Memory and context](memory-and-context.md) | What carries between sessions, and what happens when context fills up |
| [Permission approvals](permissions.md) | The four templates, the approval surface, and remembered scopes |
| [Personalization](personalization.md) | About you, response style, cross-session memory |
| [Expert roles](expert-roles.md) | Role fields, responsibilities, and review policy |
| [Manage Skills](skill-management.md) | Installing Skills, binding them to an Agent, drift notices, evolution evidence |
| [Index workspace code](code-indexing.md) | Vector indexing of workspace code |
| [Use live LSP code intelligence](lsp-code-intelligence.md) | In-session symbol navigation and diagnostics |
| [Tools and extensions](tooling.md) | MCP servers, prompt hooks, local OCR and speech extensions, Agent configurations |
| [MCP servers](mcp.md) | Connect external tools to an Agent, and approve each tool call |
| [Plugin integration](plugin-integration.md) | Built-in product integrations and readiness checks |
| [Prompt Hooks](prompt-hooks.md) | Insert content into the prompt assembly pipeline; draft, publish, roll back |
| [OnePiece (native Agent)](native-agent.md) | Usable with no CLI installed; providers, recall, and notebook editing |
| [Observability](observability.md) | Execution traces, fidelity, the log directory, and redaction |
| [Remote and IM](remote-and-im.md) | SSH remote workspaces; Feishu / DingTalk / WeCom / WeChat / Telegram |
| [Scheduled and usage](automation.md) | Running on a schedule, and how to read token usage |
| [Application updates](app-updates.md) | Release channels, signature verification, and automatic updates |

## Reference

| Chapter | What it covers |
| --- | --- |
| [Use cases](use-cases.md) | Five end-to-end scenarios, walked from the start |
| [FAQ](faq.md) | Direct answers to frequent questions |
| [Runtime and feature labels](runtime-labels.md) | How to read the "desktop only" and "Web/mock only" labels |
| [Troubleshooting](troubleshooting.md) | Start here when something breaks |
| [Reporting issues](reporting-issues.md) | Which entry point to use, what the forms need, and how to redact before submitting |

## Status labels

- **Implemented** — a user-visible path is implemented and verified.
- **Preview** — a service or mock contract exists, but the normal product workflow is incomplete.
- **Web/mock only** — deterministic browser behavior; no native side effects occurred.
- **Desktop only** — requires the Tauri runtime and local operating-system access.
- **Planned** — not yet available.

How to read each label is covered in [Runtime and feature labels](runtime-labels.md).

## What this guide does not cover

**Internal implementation and design rationale are not here.** For how the architecture is divided, why a mechanism is designed the way it is, and where the code lives, see the [VaneHub AI Developer Guide](../../../developer-guide/src/index.md) — written for developers and contributors, with architecture decisions recorded in the `src-tauri/ARCHITECTURE.md` that the [native architecture inventory](../../../developer-guide/src/index.md) points at.

The division of labour:

| What you want to know | Where |
| --- | --- |
| What happens when I press this button | This guide |
| What to do when something breaks | This guide's [Troubleshooting](troubleshooting.md) |
| Why this feature is designed this way | The [Developer Guide](../../../developer-guide/src/index.md) |
| Which file the code is in | The [Developer Guide](../../../developer-guide/src/index.md) |
| What a protocol like MCP, LSP, or RAG actually is | [Agent infrastructure technical documentation](../../../agent-infrastructure/README.md) (Simplified Chinese) |
| I found a problem, how do I report it | This guide's [Reporting issues](reporting-issues.md) |

## Simplified Chinese

The Simplified Chinese user guide covers the same full set of chapters as this one. See <a href="../../zh-CN/src/index.md">简体中文</a>.

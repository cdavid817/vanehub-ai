# VaneHub AI User Guide

<a href="../../zh-CN/src/index.md">简体中文</a>

VaneHub AI is a desktop workbench for running, managing, and orchestrating multiple AI coding agents: it drives external CLIs such as Claude Code and Codex CLI, and ships OnePiece, a built-in native agent that needs no CLI at all.

This guide is written for **individuals and teams using VaneHub AI for software development work**. To develop, test, or contribute to VaneHub AI itself, read the [Developer Guide](../../../developer-guide/src/index.md) instead.

> This page offers entry points by task; the sidebar carries the complete chapter list.

## First use

Pick one execution path first:

| Path | Who it fits | Recommended route |
| --- | --- | --- |
| **OnePiece (native agent)** | You would rather install no coding CLI | Install the app → configure a model provider → create a session → send a task |
| **External CLI agent** | You already use Claude Code, Codex CLI, Gemini CLI, OpenCode, or Antigravity CLI | Install the app → install and authenticate the CLI → check detection → create a session |

Both routes start from these chapters:

- **Download and install**: prebuilt packages and verification are in the [README's download section](../../../../README.md#download-platforms-and-release-integrity)
- [Quick Start](quick-start.md) — the shortest path to your first task
- [Install and authenticate a CLI](getting-started.md) — installation, authentication, and detection for the external-CLI route (the OnePiece route can skip this)
- [OnePiece (native agent)](native-agent.md) — model-provider configuration for the OnePiece route
- [Create your first session](first-session.md) — choosing an agent and a workspace, and meeting the session workspace
- [User interface](user-interface.md) — the main window layout, session list, conversation area, floating assistant, notifications, and tray
- [Session workspace](session-workspace.md) — the conversation, changes, files, Shell, logs, and trace areas inside a session
- [Settings center](settings.md) — the global settings entry and per-page navigation
- [Core concepts](core-concepts.md) — what session, seat, workspace, permission template, Loop, and MCP each mean

## Find by goal

### Interface and workspaces

| I need to… | Start here | Related topics |
| --- | --- | --- |
| Learn the main window and its areas | [User interface](user-interface.md) | [Create your first session](first-session.md) |
| See conversation, changes, Shell, logs, and traces in a session | [Session workspace](session-workspace.md) | [Create your first session](first-session.md) |
| Configure the app itself (language, theme, proxy, data directory) | [Settings center](settings.md) | [Application updates](app-updates.md) |
| Let an agent edit code in its own working copy | [Git worktrees](worktree.md) | [Create your first session](first-session.md) |
| Work on a remote machine | [Remote workspaces and SSH](remote-workspaces.md) | — |

### Agents and collaboration

| I need to… | Start here | Related topics |
| --- | --- | --- |
| Use the built-in agent with no CLI installed | [OnePiece (native agent)](native-agent.md) | [Quick Start](quick-start.md) |
| Install, authenticate, and manage external CLIs | [Install and authenticate a CLI](getting-started.md) | [Agent and CLI configuration](agent-configuration.md) |
| Have several agents collaborate in one session | [Multi-Agent group chat](multi-agent-workflow.md) | [Expert roles](expert-roles.md) |
| Review an agent's changes line by line | [Code review](code-review.md) | — |
| Compare agents on the same task | [Agent evaluation](evaluation.md) | — |

### Context and code intelligence

| I need to… | Start here | Related topics |
| --- | --- | --- |
| Carry memory across sessions and understand compaction | [Memory and context](memory-and-context.md) | [Personalization](personalization.md) |
| Set response style and about-you information | [Personalization](personalization.md) | [Expert roles](expert-roles.md) |
| Build a code index for a workspace | [Index workspace code](code-indexing.md) | [LSP code intelligence](lsp-code-intelligence.md) |
| Navigate symbols and diagnostics in a session | [LSP code intelligence](lsp-code-intelligence.md) | — |

### Tools and integrations

| I need to… | Start here | Related topics |
| --- | --- | --- |
| Connect external tools to an agent | [MCP servers](mcp.md) | [Agent and CLI configuration](agent-configuration.md) |
| Install Skills and bind them to an agent | [Manage Skills](skill-management.md) | — |
| Insert content into prompt assembly | [Prompt Hooks](prompt-hooks.md) | [Agent and CLI configuration](agent-configuration.md) |
| Use local OCR, speech recognition, and synthesis | [Local media](local-media.md) | [Local extensions](extensions.md) |
| Connect products such as GitHub | [Plugin integration](plugin-integration.md) | — |
| Trigger sessions from Feishu, DingTalk, and other IMs | [IM connectors](im-connectors.md) | — |
| Switch tabs and flip switches from the input box | [Slash commands](slash-commands.md) | — |

### Governance and operations

| I need to… | Start here | Related topics |
| --- | --- | --- |
| Control what an agent may do and handle approvals | [Permission approvals](permissions.md) | — |
| Let an agent iterate automatically toward a goal | [Loop Engineering](loop-engineering.md) | [Goals and the work board](goals-and-work-board.md) |
| Track goals and to-dos | [Goals and the work board](goals-and-work-board.md) | — |
| Run tasks on a schedule | [Scheduled tasks and notifications](scheduled-tasks.md) | — |
| Check token usage | [Usage statistics](usage-statistics.md) | — |
| Inspect execution traces and logs | [Observability](observability.md) | [Troubleshooting](troubleshooting.md) |
| Update the application | [Application updates](app-updates.md) | — |

## Feature availability

Some features carry platform or dependency constraints: plugin integration is desktop-only, external CLI agents need their CLI installed and authenticated, OnePiece and memory extraction need a configured model provider, and local media needs model files you supply. Each feature page's notes-and-limits section states its specific constraints.

## Getting help

| Situation | Where to go |
| --- | --- |
| I don't know how to operate a feature | [FAQ](faq.md) |
| The app or an agent misbehaves | [Troubleshooting](troubleshooting.md) |
| I found a defect or a security issue, or want to propose a feature | [Reporting issues](reporting-issues.md) |
| I want an end-to-end walkthrough | [Use cases](use-cases.md) |

## Scope of this guide

This guide covers what you can do, how to operate it, how to judge the result, and how to recover from failure. For how the architecture is divided and why a mechanism is designed the way it is, see the [Developer Guide](../../../developer-guide/src/index.md); to learn what protocols such as MCP, LSP, or RAG are in themselves, see the [agent infrastructure technical documentation](../../../agent-infrastructure/README.md) (Simplified Chinese).

## Simplified Chinese

The Simplified Chinese and English user guides share the same overall table of contents; individual chapters may briefly differ in translation progress. See <a href="../../zh-CN/src/index.md">简体中文</a>.

# User interface

**Status: Implemented — the interface is identical on desktop and in Web/mock; execution-type operations are desktop only.**

VaneHub AI's interface is one React codebase serving two runtimes: the desktop application and the browser preview. **They look the same**, so you cannot tell which runtime you are in by appearance — see [Runtime and feature labels](runtime-labels.md). Anything involving execution (starting a CLI, writing files, connecting over SSH) only really happens on desktop; in the browser preview it is simulated.

This chapter walks the interface feature by feature: what each one is and how to use it.

## Session management

### Create a session

Select **New** to open the create-session dialog, then choose the session type (Single Agent / Multi Agent), the Agent, the workspace (Local/Remote), the project folder, and the session name. A Git project is marked **Git** and can create a worktree. For Multi Agent, assign seats — see [Multi-Agent group chat](multi-agent-workflow.md).

![English create-session dialog using synthetic VaneHub Demo project data](assets/screenshots/create-session-en.png)

### Session list

The session list on the left supports three display modes: **list / by category / by project**. You can search by name or content, filter by Agent, select in bulk, reorder by dragging, and filter favorites. **Right-click a session** to rename, delete, archive, export, pin, or assign a category.

### Focus mode

**Focus mode** in the top bar collapses the session list on the left and the info panel on the right so the workspace fills the window; select it again to restore. The top bar also has **global search**, which searches messages and content across sessions.

### Activity bar navigation

The activity bar to the left of the session list switches between the main destinations: **Sessions / Loops / Plan execution / Goal Center / Todo Board / Evaluations / Scheduled tasks / Settings / Help**.

## Conversation

### Send a message

Write your task in the input box at the bottom of the workspace: **Enter sends, Shift+Enter inserts a newline.** A row of selectors and switches sits above the input box:

| Control | What it does |
| --- | --- |
| Provider / Model / Agent dropdowns | Switch among the options you have configured |
| Interaction mode / reasoning depth / configuration | Adjust parameters for this conversation |
| Streaming switch | Whether replies stream |
| Chain-of-thought switch | Whether thinking content is shown |
| Enhance button | Runs an enhancement pass over the prompt |
| File references / attachments | Attach files to the Agent as context |

### Read replies

Agent replies render rich content: code blocks with syntax highlighting, Mermaid diagrams rendered inline, thinking blocks shown collapsed, tool calls showing the tool name with its arguments and result, and images, audio, cards, checklists, and diffs rendered by type. In a long conversation a **back to bottom** button jumps to the latest message; a **welcome screen** appears the first time you enter.

### Turn status

A **turn status bar** sits at the top of the conversation area: who currently holds the turn, how long it has been waiting on a human, turn completion, and chain-depth notices. During a multi-Agent handoff it shows `handoff 1/15`. See [Multi-Agent group chat](multi-agent-workflow.md) for detail.

## Workspace tabs

Once a session is open, nine tabs sit across the top of the workspace:

| Tab | What it does |
| --- | --- |
| **Workspace** | The conversation with the Agent; the default tab |
| **Changes** | Which files the Agent changed, with a diff view (unified/split toggle, per-file review, Git status) |
| **Documents** | Browse documents inside the workspace |
| **Files** | Browse workspace files |
| **Terminal** | Commands the Agent ran, and their output |
| **Shell** | An interactive terminal for your own use |
| **Logs** | Logs for this session; searchable and seekable by time |
| **Traces** | Execution tracing (run list + span tree + per-seat tracing) — see [Observability](observability.md) |
| **Report** | Token usage (input/output/character count), a token distribution bar, and counts by message state |

**The Terminal tab and the Shell tab are not the same thing**: the first records what the Agent did, the second is a terminal for you to type in. The Agent also has a **dedicated terminal** separate from your Shell. The numeric badge on a tab is the record count; when there is a lot of data, loading is bounded and only part of the results may be shown, which the interface tells you.

## See what the Agent changed

The **Changes** tab shows the Agent's file edits:

- A file list with Git status (added/modified/deleted)
- Select a file to see its diff
- Toggle between **unified** and **split** diff views
- Review file by file

## Session information and run state

The info panel on the right shows session information, CLI tool, run state, the model used for this session, and token usage (input/output/cache read/cache write/total). There are five run states: **Idle / Starting / Running / Failed / Stopped**. With no model configured it shows "No model configured".

The info panel also carries two in-place tabs, so you do not have to jump to the settings center:

- **Skill** — view and manage the Skills bound to this session, in the session
- **Code Index** — view the workspace code index status, in the session

## Show and hide panels

The **overflow menu** (⋯) at the top right of the workspace toggles panel visibility: the session list, the info panel, and the display switch for each workspace tab.

## Session recovery

When you reopen a session after a crash or an abnormal exit, a **recovery banner** appears at the top explaining that the session was reconciled, quarantined, or needs your explicit acknowledgement.

## Settings center

**Settings** in the activity bar opens the settings center: navigation on the left, the configuration page on the right. There are 18 settings pages:

| Settings page | What it holds |
| --- | --- |
| **Basic Configuration** | Interface language, theme, font size, default policy template; launch at login, floating assistant switch; node information, network proxy including authentication, data directory, log directory, folder openers |
| **CLI Management** | Install detection, conflict diagnostics, and upgrades for each CLI — see [Install and authenticate a CLI](getting-started.md) |
| **CLI Parameters** | Launch flags per CLI Agent — see [Tools and extensions](tooling.md) |
| **SDK Dependencies** | Version management for the managed SDKs — see [Tools and extensions](tooling.md) |
| **Extension Capabilities** | Installing and enabling local multimodal capabilities — see [Tools and extensions](tooling.md) |
| **Plugin Integrations** | Integration configuration for third-party plugins |
| **MCP Servers** | MCP server configuration and per-Agent binding — see [Tools and extensions](tooling.md) |
| **Agent Configurations** | Model, policy template, and runtime parameters per Agent; you can navigate to a specific Agent, including OnePiece |
| **Agent Policies** | Permission policy and approval templates — see [Permission approvals](permissions.md) |
| **Expert Roles** | Roles and review policy — see [Personalization](personalization.md) |
| **Personalization** | Custom instructions and cross-session memory — see [Personalization](personalization.md) |
| **Skills** | Skill installation and binding — see [Manage Skills](skill-management.md) |
| **Prompt Hooks** | Hook management — see [Tools and extensions](tooling.md) |
| **IM Connectors** | IM connector configuration — see [Remote and IM](remote-and-im.md) |
| **SSH Connections** | Saved SSH connections — see [Remote and IM](remote-and-im.md) |
| **Execution Observability** | Execution tracing and log collection policy — see [Observability](observability.md) |
| **Usage Statistics** | Token usage statistics — see [Scheduled and usage](automation.md) |
| **About** | Version, update check, changelog, and repository links — see [Application updates](app-updates.md) |

## Floating assistant

Once the floating assistant is enabled in settings, a separate floating window sits on the desktop: start a session or assistant from it without opening the main window, a status badge shows the run state, and a main action menu starts tasks quickly.

## Loop center

**Loops** in the activity bar manages Loop engineering: the run list and inspector, run controls (pause/resume/cancel/accept/reject), the verification command editor, and the timeline. For the concept and how to create one, see [Loop Engineering](loop-engineering.md).

## Plan center

**Plan execution** in the activity bar opens the plan center: generate a plan draft from a goal, review/approve/run a plan, and open the plan run view.

## Notifications

The bell icon in the top bar opens the **notification center**: unread badge, mark all read, clear notifications. For notification scope (global/session) and the four notification kinds, see [Scheduled and usage](automation.md).

## System tray

On desktop there is a system tray icon: show/hide the main window, with the launch-at-login switch under **Settings → Basic settings**, and tray notifications tied to system notifications.

## Related

- Unfamiliar terminology → [Core concepts](core-concepts.md)
- First time using it → [Create your first session](first-session.md)
- Desktop versus browser preview → [Runtime and feature labels](runtime-labels.md)

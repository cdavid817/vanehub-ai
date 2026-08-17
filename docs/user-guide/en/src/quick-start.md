# Quick Start

**Status: Implemented — desktop.**

Five minutes from nothing to your first Agent run. If a CLI is already installed, start at step 2.

## 1. Prepare a CLI

VaneHub AI **drives coding Agent CLIs you have already installed**; it does not hold provider credentials itself. Install at least one:

```powershell
npm install -g @anthropic-ai/claude-code
```

Run it once in an ordinary terminal, sign in, and confirm it accepts a prompt:

```powershell
claude
```

Install the other CLIs — Codex CLI, Gemini CLI, OpenCode, Antigravity CLI — following their own official instructions. See [Install and authenticate a CLI](getting-started.md).

## 2. Confirm VaneHub AI detects it

Open **Settings → CLI management** and check the status of the CLI you installed.

If it shows as undetected, the usual cause is that the `PATH` visible to the desktop application differs from the one in your terminal — see [Troubleshooting](troubleshooting.md).

## 3. Create your first session

1. Select **New**.
2. For **Session Type**, choose **Single Agent**.
3. Pick an available Agent under **Agent**.
4. For **Workspace**, choose **Local**, then pick a directory under **Project Folder** — use **Browse**, or choose from **Recently opened projects**.
5. Fill in the session name and select **Create**.

The session enters the `idle` state once created, and you can start the conversation.

## 4. Work in the session workspace

The interface has three regions: the **session list** on the left, the **workspace** in the middle, and the **info panel** on the right (session, CLI tool, run state, model for this session, workspace path).

There are nine tabs across the top:

| Tab | What it is for |
| --- | --- |
| **Workspace** | The main surface: talk to the Agent and watch its CLI terminal |
| **Changes** | Git changes produced by this session |
| **Documents** / **Files** | Browse the working directory |
| **Terminal** | A record of the Agent's tool executions |
| **Shell** | A separate interactive terminal |
| **Logs** | Session logs, searchable and seekable by time |
| **Traces** | Execution tracing |
| **Report** | Token usage and a tool ranking |

Write your task in the input box on the **Workspace** tab. **Enter sends, Shift+Enter inserts a newline.**

![The session workspace with nine tabs across the top and the info panel on the right](assets/screenshots/session-workspace-en.png)

## 5. What next

| What you want | Where to go |
| --- | --- |
| Have several Agents collaborate in one session | [Multi-Agent group chat and `@` handoff](multi-agent-workflow.md) |
| Have an Agent iterate until the tests pass | [Loop Engineering](loop-engineering.md) |
| Limit what an Agent is allowed to do | [Permission approvals](permissions.md) |
| Have an Agent remember your preferences | [Personalization](personalization.md) |
| Understand the terminology | [Core concepts](core-concepts.md) |

## Notes

- **Provider credentials always stay in each CLI's own storage.** VaneHub AI never asks you for a provider password.
- **The browser preview (Web/mock) executes no local commands.** The interface looks operable, but it starts no process and writes no database. See [Runtime and feature labels](runtime-labels.md) for how to tell which runtime you are in.

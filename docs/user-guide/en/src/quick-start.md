# Quick Start

Five minutes from nothing to your first Agent run. If a CLI is already installed, start at step 2.

## 1. Prepare a CLI

VaneHub AI **drives coding Agent CLIs you have already installed**; it does not custody any vendor's subscription login itself. Install at least one. There are two ways:

**Method A: install it inside VaneHub AI (recommended)**

Open **Settings → CLI management**; each CLI shows an action based on its status: **Install**, **Upgrade**, **Downgrade**, **Up to date**, **Unavailable**, or **Handle manually**. Click **Install**, and VaneHub AI installs it for you through npm, then refreshes detection.

> Antigravity CLI has no npm package, so the UI offers no install/upgrade action for it — it can only go through Method B's official install script.

**Method B: install it by hand**

```powershell
npm install -g @anthropic-ai/claude-code
```

Install the other CLIs — Codex CLI, Gemini CLI, OpenCode, Antigravity CLI — following their own official instructions. See [Install and authenticate a CLI](getting-started.md).

## 1.5 Authenticate / configure a model

Two things are easy to conflate; keep them apart: **signing in** proves who you are to a vendor, and **configuring a model** decides which endpoint and which model a given CLI talks to. VaneHub AI can handle the latter; it cannot handle the former.

| | Vendor subscription login (OAuth) | Configuring a third-party model |
| --- | --- | --- |
| **External CLI** | Only in the terminal | **Can be done in VaneHub AI** |
| **Native Agent OnePiece** | Not applicable | Done in VaneHub AI |

### Vendor login: done in the terminal

VaneHub AI never walks you through a vendor's own OAuth flow, and never stores the session credentials it produces. Run it once in an ordinary terminal first and complete authentication:

```powershell
claude
```

Follow the prompts to complete your Anthropic subscription login. Codex CLI, Gemini CLI, and OpenCode work the same way, each with its own login command; Antigravity CLI goes through Google sign-in and stores credentials in the system keychain. **A CLI that doesn't work in the terminal won't work in VaneHub AI either.**

### Third-party models: configured inside VaneHub AI

If you'd rather not use an official subscription, and want a CLI to call a compatible endpoint like DeepSeek, OpenRouter, or Zhipu GLM instead, **you don't have to hand-edit any configuration file.** Open **Settings → Agent configurations**, select the target CLI, pick an entry from the built-in 25-provider catalog to save as a configuration, fill in the API key, and apply. VaneHub AI writes the relevant fields into that CLI's own global configuration file and leaves everything unrelated to it untouched.

How far each CLI goes isn't the same across the board:

| Agent | Third-party endpoint | Managed configuration file |
| --- | --- | --- |
| **Claude Code** | Supported | `~/.claude/settings.json` |
| **Codex CLI** | Supported | `~/.codex/config.toml` |
| **OpenCode** | Supported | `~/.config/opencode/opencode.json` |
| **Gemini CLI** | The endpoint can be changed, but the catalog only ships Google's official preset | `~/.gemini/.env` |
| **Antigravity CLI** | **Not supported** | `~/.gemini/antigravity-cli/settings.json` |

> **Antigravity CLI does not accept a custom endpoint.** It only goes through Google sign-in, with credentials stored in the system keychain — the configuration panel has no endpoint or key field at all. What you can adjust is the model and approval behavior.

The full field list, where credentials are stored, and how drift is handled are covered in [Tools and extensions → Agent configurations](agent-configuration.md#agent-configurations).

### The native Agent OnePiece

You can also use VaneHub AI without installing any CLI at all. In the same **Settings → Agent configurations**, open the OnePiece configuration panel: pick a vendor from the same 25-provider catalog, or fill in a custom compatible endpoint; enter an API key — **it's actually called once to validate before saving, and a failed validation is not saved**; once validated, the available model list is fetched, and you pick one. VaneHub AI stores this API key. See [Native API Agent](native-agent.md) for details.

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

![The session workspace with nine tabs across the top and the info panel on the right](assets/screenshots/session-workspace-en.png)

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

## 5. What next

| What you want | Where to go |
| --- | --- |
| Have several Agents collaborate in one session | [Multi-Agent group chat and `@` handoff](multi-agent-workflow.md) |
| Have an Agent iterate until the tests pass | [Loop Engineering](loop-engineering.md) |
| Limit what an Agent is allowed to do | [Permission approvals](permissions.md) |
| Have an Agent remember your preferences | [Personalization](personalization.md) |
| Understand the terminology | [Core concepts](core-concepts.md) |

## Notes

- **Credentials from a vendor subscription login always stay in each CLI's own storage.** VaneHub AI never takes custody of them, and never asks you for your subscription account password.
- **A third-party API key you enter in Agent configurations is held by VaneHub AI**, stored in the operating system's credential service, never written to SQLite; the UI only ever echoes back "configured."

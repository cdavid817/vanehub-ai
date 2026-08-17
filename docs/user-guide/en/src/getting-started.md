# Install and authenticate a CLI

**Status: Implemented — desktop setup.**

VaneHub AI **drives CLIs you have already installed**. It does not install models for you and does not hold provider credentials. Authentication is always done by each CLI itself, and VaneHub AI never asks you for a provider password.

The one exception is OnePiece — the built-in native API Agent, which needs no CLI at all and whose API key VaneHub AI does store. To skip CLIs and start straight away, see [Native API Agent](native-agent.md).

## Prerequisites

- Node.js 22+ and npm
- At least one supported CLI, and its subscription or API credentials

## The five CLIs

| Agent | Command | npm package | Other install channels |
| --- | --- | --- | --- |
| Claude Code | `claude` | `@anthropic-ai/claude-code` | Installer script, winget (`Anthropic.ClaudeCode`) |
| Codex CLI | `codex` | `@openai/codex` | — |
| Gemini CLI | `gemini` | `@google/gemini-cli` | — |
| OpenCode | `opencode` | `opencode-ai` | Installer script |
| Antigravity CLI | `agy` | none | Installer script only (`install.sh` on Unix, `install.ps1` on Windows) |

Installing one is enough to start; you do not need all five.

> **Antigravity CLI ships no npm package.** It can only be installed through the official installer script, so the CLI Management page offers no npm upgrade or downgrade action for it.

```powershell
npm install -g @anthropic-ai/claude-code
```

## Get it working in a terminal first

**After installing, run it once in an ordinary terminal and complete its authentication**, confirming that it accepts a prompt:

```powershell
claude
```

Do not skip this step. What VaneHub AI detects is whether the command can run; it cannot complete a provider's sign-in flow on your behalf. **A CLI that does not work in your terminal will not work inside VaneHub AI either.**

## Read the detection status

Settings → **CLI Management** shows the status of each CLI. **There are six, and they mean very different things:**

| Status | Meaning | What to do |
| --- | --- | --- |
| **Installed** | The executable resolves on this machine | Nothing |
| **Not Installed** | No executable resolves on this machine | Install it per the table above |
| **Installed but not runnable** | The file was found, but executing it fails | See below |
| **Installation conflict** | Multiple installations detected | See below |
| **Unsupported** | This install channel is not supported on the current platform | Use another source |
| **Undetected** | Detection has not run yet | Refresh detection |

**Do not try to fix "Installed but not runnable" by reinstalling.** The interface says so directly:

> The active CLI is installed but cannot run. Check Node, PATH, or the tool environment; reinstalling the same version usually will not repair it by itself.

The cause is normally the Node version, `PATH`, or the CLI's own runtime environment — not a missing file.

## Installation conflicts

**"Installation conflict" means the same CLI is installed more than once** — for example once globally through npm and again through an installer script or winget.

Select **Diagnose Conflicts** to expand **Installation diagnostics**, which lists every local installation path it found and marks the **Active** one.

The guidance in the interface reads:

> Multiple installations were detected. Expand installation diagnostics to confirm the active path; upgrades should target the command-line default.

**Why this matters:** if you upgrade the wrong copy, the command line still resolves the old one, and it looks as though the upgrade did nothing.

**The source has to match.** If the active copy was not installed through npm, the interface says:

> The active path comes from {source}. Use that source's update path; VaneHub will not add another npm copy and present it as an upgrade.

This is deliberate — installing another copy through npm would only make the conflict worse.

## Available actions

The CLI Management page offers different actions depending on status: **Install**, **Upgrade**, **Downgrade**, **Current Version**, **Unavailable**, and **Manual action**.

"Manual action" means VaneHub AI has judged that it should not act on this state automatically:

> This installation state needs manual handling. Refresh detection and review installation diagnostics before choosing the source-native repair path.

## Authentication

**Authentication does not happen inside VaneHub AI.** The five CLIs each manage their own provider credentials, stored in their own locations.

If an Agent asks you to sign in during a session, complete authentication in that CLI, then return to VaneHub AI and refresh detection.

## Web preview

**Status: Web/mock only.** The browser preview shows deterministic availability and execution fixtures. It **neither detects nor authenticates local CLIs**. Seeing "Installed" does not mean anything is installed on your machine.

See [Runtime and feature labels](runtime-labels.md) for how to tell which runtime you are in.

## Next

Once a CLI works, go to [Create your first session](first-session.md).

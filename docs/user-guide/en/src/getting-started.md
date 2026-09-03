# Install and authenticate a CLI

VaneHub AI **drives CLIs you have already installed**. **Each vendor's subscription login (OAuth) is always completed by the CLI itself** — VaneHub AI never does it for you, and never asks for your subscription account password.

But **"configuring a third-party model" is a separate matter** — if you want a given CLI to call a compatible endpoint like DeepSeek or OpenRouter, you can configure and apply that under **Settings → Agent configurations**, without hand-editing any file. See [Quick Start → Authenticate / configure a model](quick-start.md#15-authenticate--configure-a-model) for how the two divide.

You can also start without installing any CLI at all: OnePiece is the built-in native API Agent, needing no CLI. See [Native API Agent](native-agent.md).

## Prerequisites

- Node.js 22+ and npm
- At least one supported CLI, and its subscription or API credentials

## Two ways to install

There are two routes to installing a CLI, and **they produce the same result — the difference is who runs the install command.**

### Method A: install it inside VaneHub AI

Open **Settings → CLI management**; each CLI card offers whatever its source supports — install, upgrade, downgrade, or nothing at all. Pick a version, review the plan VaneHub shows you, and confirm; detection refreshes when it finishes and reports whether the change was verified.

Good for: you already have Node.js 22+ on this machine, and you're fine with the CLI coming from npm or, on Windows, from WinGet.

**Two things to know going in:**

- **The source decides what is possible.** VaneHub AI drives npm, WinGet on Windows, and per-CLI audited vendor installers. Homebrew, Bun, Volta, desktop bundles, and system packages are detected and reported but never changed. It never pipes a downloaded script into a shell, and it never installs a second copy beside someone else's and calls that an upgrade.
- **Antigravity CLI has no npm package.** Its only source is the vendor installer, which pins no exact version, so the UI offers an upgrade to latest rather than a version list.

### Method B: install it from the terminal

Run the install command per each CLI's official instructions, as detailed in the next section. Afterward, go back to **Settings → CLI management** and click **Refresh detection**.

Good for: you want the officially recommended native binary (no Node.js dependency), you need a specific source like Homebrew/scoop, or the CLI has no npm package at all.

> **Don't mix the two routes.** Installing the same CLI once via npm and once via an install script triggers an [installation conflict](#installation-conflicts) — at that point `PATH` order decides which copy actually runs, and an upgrade often lands on the other one.

Whichever route you take, **authentication always has to happen in the terminal**; see [Get it working in a terminal first](#get-it-working-in-a-terminal-first).

## The five CLIs

VaneHub AI supports five external CLI Agents. Installing one is enough to start; you don't need all five. The table below summarizes each CLI's install method; each subsection gives the exact commands.

| Agent | Provider | Command | Dependency | Recommended install |
| --- | --- | --- | --- | --- |
| Claude Code | Anthropic | `claude` | None (native binary); npm needs Node.js 22+ | Native install script |
| Codex CLI | OpenAI | `codex` | None (native binary); npm needs Node.js 18+ | One-line install script |
| Gemini CLI | Google | `gemini` | Node.js 18+ | npm global install |
| OpenCode | sst (open source) | `opencode` | None (native binary); npm needs Node.js | One-line install script |
| Antigravity CLI | Google | `agy` | None (single Go binary) | One-line install script |

### Claude Code

Anthropic's official command-line coding assistant; its model family in VaneHub AI is Anthropic. Requires a Claude Pro / Max / Team / Enterprise account, or Anthropic Console API credit. The official recommendation is now **native binary install** (no Node.js dependency; the npm package actually downloads the same native binary):

```bash
# macOS / Linux
curl -fsSL https://claude.ai/install.sh | bash
# Windows (PowerShell)
irm https://claude.ai/install.ps1 | iex
# npm (still works, needs Node.js 22+; never use sudo npm install -g — use nvm or adjust the npm global prefix instead)
npm install -g @anthropic-ai/claude-code
```

Authenticate by running `claude` in a terminal and following the prompts (a browser login opens the first time). VaneHub AI determines availability through `claude-sdk` or `claude` on PATH, and **never stores your credentials**. Verify with `claude --version` and `claude doctor`.

### Codex CLI

OpenAI's official CLI; its model family is OpenAI. Requires an OpenAI account (Plus / Pro / Business / Edu / Enterprise plan, or an API key):

```bash
# macOS / Linux
curl -fsSL https://chatgpt.com/codex/install.sh | sh
# Windows (PowerShell)
irm https://chatgpt.com/codex/install.ps1 | iex
# npm (needs Node.js 18+)
npm install -g @openai/codex
# Homebrew (macOS)
brew install --cask codex
```

Authenticate by running `codex` in a terminal and selecting "Sign in with ChatGPT." The install script downloads from `releases.openai.com` by default, falling back to GitHub Releases on failure (set `CODEX_INSTALLER_USE_RELEASES_OPENAI_COM=false` to force GitHub). Verify with `codex --version`.

### Gemini CLI

Google's official CLI; its model family is Google. Authenticates with a Google account (OAuth):

```bash
npm install -g @google/gemini-cli
```

Authenticate by running `gemini` in a terminal and selecting "Login with Google." The free personal-account quota is roughly 60 requests per minute and 1000 per day.

> **Gemini CLI is being phased out for personal users.** Google has announced a migration from Gemini CLI to Antigravity CLI: starting 2026-06-18, Gemini CLI and Gemini Code Assist are being phased out for personal/free users (Free / Pro / Ultra tiers), with the official recommendation to migrate to [Antigravity CLI](#antigravity-cli). Enterprise Gemini Code Assist Standard/Enterprise and paid API key channels are unaffected.

### OpenCode

An open-source CLI (`sst/opencode`) that supports many providers; its model family in VaneHub AI is Unknown. Note that the identically named `opencode-ai/opencode` on GitHub (Go/Bubble Tea) is a different, unrelated project — VaneHub AI integrates with `sst/opencode`:

```bash
# macOS / Linux (one-line script)
curl -fsSL https://opencode.ai/install | bash
# npm / bun / pnpm / yarn
npm i -g opencode-ai@latest
# Homebrew
brew install sst/tap/opencode
# Windows
scoop bucket add extras && scoop install extras/opencode
```

Authentication depends on which provider you choose, configured after running `opencode` in a terminal. Note: OpenCode doesn't support long context, and VaneHub AI adjusts its context capability accordingly.

### Antigravity CLI

Google's official CLI (the successor to Gemini CLI), whose command is `agy` (not `antigravity`); its model family is Google. **It has no npm package** and can only be installed through the official install script:

```bash
# macOS / Linux
curl -fsSL https://antigravity.google/cli/install.sh | bash
# Windows (PowerShell)
irm https://antigravity.google/cli/install.ps1 | iex
# Windows (cmd)
curl -fsSL https://antigravity.google/cli/install.cmd -o install.cmd && install.cmd && del install.cmd
```

The binary lands in `~/.local/bin` (macOS/Linux) or `%LOCALAPPDATA%\Antigravity\` (Windows) by default, which is why the CLI Management page offers no npm install/upgrade/downgrade action for it. It goes through **Google sign-in** and stores credentials in the **system keychain** — the configuration panel has no key field at all. If Gemini CLI was previously installed on this machine (a `~/.gemini` directory exists), `agy`'s first run offers to import the old settings (MCP configuration, command allowlist, shortcuts, theme); it doesn't conflict with an npm-installed Gemini CLI, and both can coexist.

> **Subscription logins are always self-managed by each CLI.** VaneHub AI only checks "can this command run" — it never completes an OAuth login for you, and never stores the session credentials that produces. (A third-party API key you actively enter under **Settings → Agent configurations** is a different matter — VaneHub AI stores that in the system credential service.) After installing, it's a good idea to run `claude --version` / `codex --version` / `gemini --version` / `opencode --version` / `agy --version` and confirm a normal version number before adding a session in VaneHub AI.

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

The CLI Management page offers different actions depending on status: **Install**, **Upgrade**, **Downgrade**, **Up to date**, **Unavailable**, **Handle manually**.

"Handle manually" means VaneHub AI has judged that it should not act on this state automatically:

> This installation state needs manual handling. Refresh detection and review installation diagnostics before choosing the source-native repair path.

## Authentication

**Vendor subscription login does not happen inside VaneHub AI.** The five CLIs each manage their own subscription credentials, stored in their own locations.

If an Agent asks you to sign in during a session, complete authentication in that CLI, then return to VaneHub AI and refresh detection.

**Switching to a third-party model works the other way around** — build and apply a configuration under **Settings → Agent configurations**, without hand-editing the CLI's own file. See [Tools and extensions → Agent configurations](agent-configuration.md#agent-configurations).

## CLI launch parameters

Each CLI's own command-line parameters and how to configure launch parameters inside VaneHub AI are collected under [Tools and extensions → CLI parameters](agent-configuration.md#cli-parameters). OnePiece has no CLI and therefore no launch parameters; its equivalent configuration lives under [Agent configurations](agent-configuration.md#agent-configurations).

## Next

Once a CLI works, go to [Create your first session](first-session.md).

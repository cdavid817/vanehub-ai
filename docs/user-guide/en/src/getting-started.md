# Install and authenticate a CLI

**Status: Delivered — desktop setup.**

VaneHub AI drives installed coding-agent CLIs. Authentication remains in each provider's CLI; VaneHub AI does not ask for your provider password.

## Prerequisites

- Node.js 22+ and npm
- VaneHub AI or the repository development environment
- At least one supported CLI and its subscription or API credentials

Install one provider:

```powershell
npm install -g @anthropic-ai/claude-code
```

```powershell
npm install -g @openai/codex
```

Gemini CLI and OpenCode can be installed with their official instructions.

Run the chosen command once in a regular terminal and complete its authentication:

```powershell
claude
```

```powershell
codex
```

Verify that the CLI accepts a prompt before opening it through VaneHub AI. Provider credentials remain in the CLI's own storage.

## Web preview

**Status: Web/mock only.** Browser preview shows deterministic availability and execution fixtures. It does not inspect or authenticate local CLIs.

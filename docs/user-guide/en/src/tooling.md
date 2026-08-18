# Tools and extensions

**Status: Implemented — desktop only.**

## Overview

MCP servers, prompt hooks, local extensions, plugin integrations, SDK dependencies, and CLI parameters are all configured centrally in the settings center and then handed to each Agent, rather than being configured separately inside every CLI.

Skill management has its own chapter: [Manage Skills](skill-management.md).

## MCP servers

An MCP server connects external tools to an Agent, registered centrally under **Settings → MCP Servers**. The three transports, the naming rules, connection testing and status caching, Claude Desktop import and export, the relay's scope, per-call tool approval, and the resource limits are all in [MCP servers](mcp.md).

## Prompt Hooks

A Prompt Hook inserts content into the prompt assembly pipeline, configured under **Settings → Prompt Hooks**. The seven categories, the two execution stages, the template variable allowlist, draft/publish/rollback, and evaluation are all in [Prompt Hooks](prompt-hooks.md).

> **Prompt Hooks can only be bound to the four external CLI Agents and do not apply to OnePiece** — the native Agent has its own core-instruction mechanism.

## Extension capabilities

What **Settings → Extension Capabilities** installs is **local multimodal AI capability**, not general-purpose plugins. The first release provides one built-in allowlisted framework per capability:

| Capability | Framework | Runtime | Local port | Estimated disk |
| --- | --- | --- | --- | --- |
| **OCR** | PaddleOCR | Python 3.10+ | 9875 | **~1800 MB** |
| **Speech Recognition** | faster-whisper | Python 3.10+ | 9876 | **~900 MB** |
| **Speech Synthesis** | sherpa-onnx | Python 3.10+ | — | — |

**Check two things before installing**: you need Python 3.10+ on the machine, and **the disk footprint is not small** — PaddleOCR is close to 1.8 GB. Every framework card has an expandable "installation requirements" section.

The top of the page has three counters, **Installed / Running / Errors**; when something errors, check the operation logs for the reason.

![The Extension Capabilities settings page with the PaddleOCR and faster-whisper framework cards](assets/screenshots/extensions-en.png)

## Plugin integrations

**Settings → Plugin Integrations** is for integration configuration of third-party plugins.

## SDK dependencies

**There are only two managed SDKs**: the Claude Code SDK and the Codex SDK, each corresponding to one npm package and carrying three alternative versions — so you can fall back when a version misbehaves.

Gemini CLI, OpenCode, and Antigravity CLI have no corresponding managed SDK.

## CLI management and parameters

### CLI management

**Settings → CLI Management** collects the installation status of the CLIs in one place, with **Installed / Not Installed** counters at the top and three actions: **Diagnose Conflicts**, refresh detection, and **Upgrade All**.

**The same CLI may come from several sources** (npm, winget, Homebrew, Volta, Bun, and others), which is exactly where conflicts come from. There are four:

| Conflict | Meaning |
| --- | --- |
| Multiple installations detected | More than one copy is installed |
| Version mismatch | The copies are at different versions |
| **What actually runs is not what you expect** | **`PATH` order decides which copy really runs** |
| No conflict | Normal |

The third is the most insidious — you believe you are using A while B is what runs.

**Whether VaneHub AI can upgrade for you depends on the install source**: with a manual installation or an unrecognized source, all it can do is tell you to handle it yourself.

![The CLI Management settings page with CLI cards and the local environment check](assets/screenshots/cli-en.png)

### CLI parameters

**Settings → CLI Parameters** configures launch flags per CLI. Parameters carry two annotations:

- **Risk annotation** — dangerous flags are marked prominently
- **Launch scenario** — distinguishing "interactive terminal" from "conversation", because the same CLI needs different parameters in the two cases

> **A policy template overrides the choices you save here.** For example, while the Read-only template is active, a permissive option ticked in the parameters still yields to the template. Security policy takes precedence over convenience configuration.

## Notes and limits

- **All of this is desktop only**; the browser preview shows mock data.
- **Drift is reported but not auto-repaired** — when configuration is detected as changed externally, you decide how to handle it.
- **It does not rewrite any CLI's own configuration files**; tool binding is achieved through launch flags and the relay.

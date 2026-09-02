<div align="center">

**English**
· [简体中文](README.zh-CN.md)
· [日本語](README.ja.md)

</div>

<!-- docs-section:hero -->

# VaneHub AI

<p align="center">
  <img src="public/icon-512.png" alt="VaneHub AI app icon" width="160" />
</p>

Desktop-first workspace for managing AI coding agents through one React interface and explicit Web/mock and Tauri runtime boundaries.

<!-- docs-fact:project-version value:1.4.0 -->
<!-- docs-fact:tauri-major value:2.x -->
<!-- docs-fact:react-major value:19.x -->

[![Version](https://img.shields.io/badge/version-1.4.0-blue.svg)](package.json)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB.svg)](src-tauri/Cargo.toml)
[![React](https://img.shields.io/badge/React-19.x-61DAFB.svg)](package.json)
[![CI](https://github.com/cdavid817/vanehub-ai/actions/workflows/ci.yml/badge.svg)](https://github.com/cdavid817/vanehub-ai/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-green.svg)](LICENSE)

<!-- docs-section:overview -->

## Overview

VaneHub AI brings Claude Code, OpenCode, Codex CLI, Gemini CLI, and Antigravity CLI into a shared desktop workspace. It manages CLI availability, sessions, terminal execution, projects and worktrees, settings, tools, observability, and desktop integrations without letting React components depend directly on native APIs.

### Supported CLIs

One is enough to start. You do not need all five.

| Agent | Provider | Command | Model family | In-app install | Third-party model endpoint |
| --- | --- | --- | --- | --- | --- |
| Claude Code | Anthropic | `claude` | Anthropic | ✅ `@anthropic-ai/claude-code` | ✅ |
| Codex CLI | OpenAI | `codex` | OpenAI | ✅ `@openai/codex` | ✅ |
| OpenCode | OpenCode (open source) | `opencode` | Unknown | ✅ `opencode-ai` | ✅ |
| Gemini CLI | Google | `gemini` | Google | ✅ `@google/gemini-cli` | ⚠️ Custom endpoint allowed, but the catalog ships only the official preset |
| Antigravity CLI | Google | `agy` | Google | ❌ No npm package; use the official installer script | ❌ Google sign-in only |

- In-app install means VaneHub AI can install and upgrade the CLI for you from Settings → CLI management. It drives npm, WinGet on Windows, and per-CLI audited vendor installers. A copy that came from Homebrew, Bun, Volta, a desktop bundle, or a system package is detected and reported but never changed — VaneHub names the tool that owns it instead of installing a second copy beside it.
- Third-party model endpoint means the CLI can be pointed at a compatible endpoint such as DeepSeek or OpenRouter from Settings → Agent configurations. **Vendor subscription login (OAuth) always happens in your terminal**; VaneHub AI does not broker it.
- OpenCode's model family is "Unknown" by decision, not omission: it drives whichever model you configured, so it has no fixed family, and policies such as "require a reviewer from a different model family" do not apply to it.
- Gemini CLI is being replaced by Antigravity CLI. Google began phasing it out for personal and free accounts on 2026-06-18.
- If you would rather install no CLI at all, the built-in native API Agent OnePiece calls model providers over HTTP entirely inside the application. See the user guide below.

### Supported model providers

Twenty-five providers ship as configuration templates, shared by OnePiece and three of the CLI Agents. Anything outside the catalog can be added as a custom compatible endpoint.

| Category | Providers |
| --- | --- |
| Official | Anthropic, OpenAI |
| Aggregators and cloud platforms | OpenRouter, SiliconFlow, Alibaba Bailian, Volcengine Ark, Together AI, Fireworks AI, NVIDIA NIM, ModelScope, PPIO, Qiniu AI |
| Model vendors | DeepSeek, Zhipu GLM, Kimi / Moonshot, xAI, Mistral AI, MiniMax, MiniMax Global, StepFun, Baichuan AI, Xiaomi MiMo, Z.AI |
| Inference accelerators | Groq, Cerebras |

**Which Agent a provider can serve depends on the endpoint protocols it offers**: the 16 providers speaking Anthropic Messages can back Claude Code, and the 24 speaking OpenAI Chat Completions can back Codex CLI and OpenCode.

The full catalog — vendor icons, endpoint protocols, default models, and API key links — is in the [built-in model provider catalog](docs/model-providers.md) (Simplified Chinese).

<!-- docs-section:download -->

## Download

Prebuilt desktop packages are published on the [Releases page](https://github.com/cdavid817/vanehub-ai/releases): a signed Windows x64 `.exe` installer, signed and notarized macOS x64 and Apple Silicon `.dmg` files, and Linux x64 and ARM64 `.deb` and AppImage builds. No `.msi` or `.rpm` is published.

Verify downloads against the published `SHA256SUMS`, SPDX SBOM, and GitHub attestations. Linux packages carry integrity and provenance evidence but do not use operating-system code signing.

<!-- docs-section:documentation -->

## Documentation

<!-- docs-locale-guides -->

### User guide

The [user guide](docs/user-guide/en/src/index.md) has the complete chapter list in its own sidebar; the table below is only the way in.

| Group | Start here | Covers |
| --- | --- | --- |
| Getting started | [Quick Start](docs/user-guide/en/src/quick-start.md) | Installing, authenticating and upgrading a CLI, a first session, core concepts |
| Interface and workspaces | [User interface](docs/user-guide/en/src/user-interface.md) | Layout and navigation, the session workspace, settings, remote workspaces, worktrees, slash commands |
| Agents and collaboration | [Multi-Agent group chat](docs/user-guide/en/src/multi-agent-workflow.md) | OnePiece, seats and handoff, expert roles, Loop, goals and the work board, code review, evaluation |
| Context and code intelligence | [Memory and context](docs/user-guide/en/src/memory-and-context.md) | Cross-session memory and personalization, compaction, code indexing, LSP |
| Tools and integrations | [Agent and CLI configuration](docs/user-guide/en/src/agent-configuration.md) | CLI parameters, providers, Skills, MCP, Prompt Hooks, local extensions, local media, IM connectors |
| Governance and operations | [Permission approvals](docs/user-guide/en/src/permissions.md) | Permission templates and approvals, observability, scheduled tasks, usage statistics |
| Help | [Troubleshooting](docs/user-guide/en/src/troubleshooting.md) | Use cases, FAQ, troubleshooting, reporting issues |

### Developer guide

The [developer guide](docs/developer-guide/src/index.md) has the complete chapter list in its own sidebar; the table below is only the way in.

| Domain | Start here | Covers |
| --- | --- | --- |
| Orientation and boundaries | [Repository orientation](docs/developer-guide/src/repository-orientation.md) | Directory ownership, runtime and service boundaries, native bounded contexts, persistence ownership |
| Agent runtime | [Agent lifecycle and provider runtime](docs/developer-guide/src/agent-lifecycle.md) | OnePiece, the tool registry, CLI lifecycle and delegation, terminal and PTY, group chat, Loop and Plan, session recovery |
| Context, memory, and code intelligence | [Cross-session memory](docs/developer-guide/src/cross-session-memory.md) | Compaction, personalization governance, retrieval, Tree-sitter indexing, LSP |
| Skills and external integrations | [Skill management](docs/developer-guide/src/skill-management.md) | The effective Skill runtime, overlay governance, evolution evidence, MCP tools, IM connectors |
| Security, evaluation, and observability | [Permission model](docs/developer-guide/src/permission-model.md) | Execution observability, the evaluation runtime, the evidence console, unified logging, usage statistics |
| Engineering delivery | [Testing](docs/developer-guide/src/testing.md) | The OpenSpec workflow, release, and live qualification |
| Generated reference | [Native API reference](docs/developer-guide/src/native-api-reference.md) | The native contract and ownership reference generated from source |

User guides are available in English and Simplified Chinese. Japanese, Traditional Chinese, and Korean are delivered as application UI resource locales only; no user guide is provided for those locales.

<!-- /docs-locale-guides -->

<!-- docs-section:architecture -->

## Architecture

```mermaid
flowchart LR
  UI[React UI] --> Service[Frontend service interfaces]
  Service --> Web[Web/mock adapters]
  Service --> Tauri[Tauri adapters]
  Tauri --> Commands[Rust commands]
  Commands --> Contexts[Native bounded contexts]
  Contexts --> SQLite[(SQLite)]
  Contexts --> CLI[Agent CLIs]
```

React components call services in `src/services/`. Tauri-specific `invoke()` calls stay in frontend Tauri adapters, while SQLite, CLI processes, filesystem access, and desktop lifecycle behavior stay in Rust.

<!-- docs-section:quick-start -->

## Run from source

<!-- docs-fact:node-minimum value:22+ -->

Prerequisites: Node.js 22+, npm, stable Rust, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform.

For platform linker requirements, release-profile behavior, worktree cache guidance, and measured build evidence, see the [native build performance guide](docs/build-performance.md).

```powershell
npm ci
```

Run Web/mock preview:

```powershell
npm run dev -- --host 127.0.0.1
```

Run the desktop application:

```powershell
$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"
npm run tauri:dev
```

Web/mock is a deterministic browser simulation. It does not claim local CLI execution, SQLite persistence, filesystem changes, or operating-system side effects.

<!-- docs-section:development -->

## Development

Run every command in the「校验命令」(validation commands) section of AGENTS.md verbatim before submitting changes; that list is the single source of truth aligned with CI.

New features and architecture changes require an OpenSpec proposal before implementation. See [AGENTS.md](AGENTS.md) and [openspec/project.md](openspec/project.md) for project rules.

### Agent infrastructure technical documentation

| Topic | Entry |
| --- | --- |
| MCP | [protocol model and three-role architecture, transports, core primitives, lifecycle, authorization and security](docs/agent-infrastructure/protocols/mcp.md) |
| Function Calling | [the call loop and constrained decoding, Anthropic versus OpenAI API differences, parallel calls and streaming assembly, structured output](docs/agent-infrastructure/protocols/function-calling.md) |
| LSP | [protocol layering and lifecycle, capability negotiation, text synchronization, language and workspace features](docs/agent-infrastructure/protocols/lsp.md) |
| A2A | [AgentCard/Task/Message/Artifact data model, task state machine, discovery, asynchronous update channels](docs/agent-infrastructure/protocols/a2a.md) |
| Multi-Agent systems | [orchestration topologies and role frameworks, communication, context management, execution isolation, failure modes](docs/agent-infrastructure/patterns/multi-agent.md) |
| Agent Skills | [the open specification and file format, progressive-disclosure loading, triggering and execution, comparison with MCP and prompts](docs/agent-infrastructure/patterns/agent-skills.md) |
| RAG | [indexing and retrieval pipelines, semantic versus keyword retrieval, hybrid retrieval and reranking, evaluation](docs/agent-infrastructure/patterns/rag.md) |
| Tree-sitter | [GLR incremental parsing, grammar toolchain and ABI, the query system, structured code chunking and repo maps](docs/agent-infrastructure/patterns/tree-sitter.md) |
| OpenSpec | [the knowledge model behind spec-driven development, change-package artifact chains, the opsx command family, delta spec merging](docs/agent-infrastructure/methods/openspec.md) |

Reference: [native architecture inventory](src-tauri/ARCHITECTURE.md) · [CLI parameter reference](docs/reference/cli/builtin-cli-reference.md) · [contributing](CONTRIBUTING.md) · [native build performance](docs/build-performance.md) · [release signing](docs/release-signing.md)

Build the mdBook guides and Rustdoc reference:

```powershell
npm run docs:check
npm run docs:test
npm run docs:build
```

The documentation build requires the mdBook version pinned in `docs/toolchain.json`.

<!-- docs-section:roadmap -->

## Roadmap

Implemented work and current contracts are recorded in [OpenSpec main specifications](openspec/specs/). Near-term product work includes custom Agents and a plugin marketplace. Local OCR, speech-to-text, and text-to-speech already run on the local machine; the remaining work there is engine and platform coverage, install automation, and real-device qualification.

<!-- docs-section:contributing -->

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a change. Keep documentation, both frontend runtime adapters, native contracts, tests, and OpenSpec artifacts aligned with the behavior you change.

<!-- docs-section:license -->

## License

Licensed under the Apache License 2.0. See [LICENSE](LICENSE).

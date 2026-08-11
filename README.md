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

<!-- docs-fact:project-version value:0.1.0-preview.1 -->
<!-- docs-fact:tauri-major value:2.x -->
<!-- docs-fact:react-major value:19.x -->

[![Version](https://img.shields.io/badge/version-0.1.0--preview.1-blue.svg)](package.json)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB.svg)](src-tauri/Cargo.toml)
[![React](https://img.shields.io/badge/React-19.x-61DAFB.svg)](package.json)
[![CI](https://github.com/cdavid817/vanehub-ai/actions/workflows/ci.yml/badge.svg)](https://github.com/cdavid817/vanehub-ai/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-green.svg)](LICENSE)

<!-- docs-section:overview -->

## Overview

VaneHub AI brings Claude Code, OpenCode, Codex CLI, Gemini CLI, and Antigravity CLI into a shared desktop workspace. It manages CLI availability, sessions, terminal execution, projects and worktrees, settings, tools, observability, and desktop integrations without letting React components depend directly on native APIs.

<!-- docs-section:download -->

## Download

Prebuilt desktop packages are published on the [Releases page](https://github.com/cdavid817/vanehub-ai/releases): a Windows `.exe` installer, a macOS `.dmg`, and Linux `.deb` and AppImage builds. No `.msi` or `.rpm` is published.

The current build is an unsigned preview. Windows and macOS warn before running it, and the release notes carry the steps for each platform. Verify your download against the published `SHA256SUMS` before installing.

<!-- docs-section:feature-status -->

## Feature status

<!-- feature:core-workspace status:delivered -->

- **Delivered:** CLI management, single-Agent sessions, interactive Agent terminals, session organization, project/worktree and SSH workspace tools, settings, MCP/SDK/Skills/Prompt Hooks/extensions, IM connectors, scheduled tasks, notifications, usage reporting, unified redacted logs, and cross-platform packaging.

<!-- feature:multi-agent-runtime status:delivered -->

- **Delivered:** Multi-Agent group chat runtime. A session holds several Agent seats, a reply hands the turn over with an `@` mention, and both mention count and chain depth are bounded. This replaces the earlier dependency-graph coordination runtime, which has been removed.

<!-- feature:multi-agent-ui status:delivered -->

- **Delivered:** Seat assignment in the normal create-session dialog, plus seat switching, speaker attribution, and turn status inside the session workspace.

<!-- feature:japanese-ui status:delivered -->

- **Delivered:** Japanese application UI resources, at key parity with the other supported locales — English, Simplified Chinese, Traditional Chinese, and Korean.

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

## Quick start

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
npm run tauri -- dev
```

Web/mock is a deterministic browser simulation. It does not claim local CLI execution, SQLite persistence, filesystem changes, or operating-system side effects.

<!-- docs-section:documentation -->

## Documentation

<!-- docs-locale-guides -->

Pick by what you are doing:

- **Using VaneHub AI** → [Install and authenticate a CLI](docs/user-guide/en/src/getting-started.md), then the rest of the [User Guide](docs/user-guide/en/src/index.md).
- **Working on it** → [Developer Guide](docs/developer-guide/src/index.md) for repository layout, runtime boundaries, bounded contexts, persistence, testing, and release.

<!-- /docs-locale-guides -->

Reference: [native architecture inventory](src-tauri/ARCHITECTURE.md) · [contributing](CONTRIBUTING.md) · [native build performance](docs/build-performance.md) · [release signing](docs/release-signing.md)

Build the mdBook guides and Rustdoc reference:

```powershell
npm run docs:check
npm run docs:test
npm run docs:build
```

The documentation build requires the mdBook version pinned in `docs/toolchain.json`.

<!-- docs-section:development -->

## Development

Run every command in the「校验命令」(validation commands) section of AGENTS.md verbatim before submitting changes; that list is the single source of truth aligned with CI.

New features and architecture changes require an OpenSpec proposal before implementation. See [AGENTS.md](AGENTS.md) and [openspec/project.md](openspec/project.md) for project rules.

<!-- docs-section:roadmap -->

## Roadmap

Delivered work and current contracts are recorded in [OpenSpec main specifications](openspec/specs/). Near-term product work includes the Multi-Agent coordination UI, persistent Agent memory, custom Agents, a plugin marketplace, and extended local OCR/speech capabilities.

<!-- docs-section:contributing -->

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a change. Keep documentation, both frontend runtime adapters, native contracts, tests, and OpenSpec artifacts aligned with the behavior you change.

<!-- docs-section:license -->

## License

Licensed under the Apache License 2.0. See [LICENSE](LICENSE).

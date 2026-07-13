# VaneHub AI

VaneHub AI is a desktop-first tool for managing and switching between AI coding agents such as Claude Code, OpenCode, Codex CLI, and Gemini CLI.

The app is built with Tauri, React, TypeScript, Rust, SQLite, and Playwright. The same React UI can run inside the Tauri desktop client or as a browser page through a Web/mock adapter.

## Architecture

- `src/` - React frontend and runtime service adapters.
- `src/services/agent-service.ts` - frontend service boundary used by UI components.
- `src/services/tauri-agent-client.ts` - Tauri desktop adapter.
- `src/services/web-agent-client.ts` - Web/mock adapter.
- `src-tauri/` - Rust/Tauri commands, SQLite registry, local CLI checks, launch routing, and session state.
- `openspec/` - OpenSpec project rules, specs, and archived change history.

React components should depend on service interfaces, not direct Tauri `invoke()` calls.

## Prerequisites

- Node.js 24+
- Rust stable
- Microsoft C++ Build Tools with MSVC and Windows SDK
- WebView2 Runtime

## Install

```powershell
npm install
```

## Run Desktop App

```powershell
$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"
npm run tauri -- dev
```

## Run Web UI

```powershell
npm run dev -- --host 127.0.0.1
```

Open:

```text
http://127.0.0.1:1420/
```

## Validate

```powershell
npm run test
npm run build
$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo test --manifest-path src-tauri\Cargo.toml
cargo check --manifest-path src-tauri\Cargo.toml
openspec validate --specs --strict
```

## Package

```powershell
npm run package
```

Generated desktop artifacts are written under `src-tauri/target/release/bundle/`.

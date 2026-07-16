## Why

The current Provider Management settings page is a demo-style API provider list backed by frontend-local data. VaneHub AI's supported tools are local AI coding CLIs, so this page should instead help users understand and manage the real local CLI environment for Claude Code, Codex CLI, Gemini CLI, and OpenCode.

The replacement must avoid mock installation data. Opening the page should stay fast by reading the last persisted detection result, while refresh, remote version checks, installation, upgrade, and downgrade run asynchronously as backend-managed operations.

## What Changes

- Replace the Provider Management page with a `CLI 管理` settings page.
- Show four CLI tools in a fixed order: Anthropic Claude Code CLI, OpenAI Codex CLI, Google Gemini CLI, and OpenCode CLI.
- Remove provider API configuration concepts from this page, including API Key, URL, presets, enable, edit, delete, active provider count, add provider, and empty provider configuration states.
- Show only two summary cards: CLI installed and CLI not installed.
- Read the last persisted CLI detection result for initial rendering without starting local command or network checks.
- On first startup when no persisted detection result exists, automatically start one asynchronous refresh for local CLI installation and latest-version metadata.
- Add asynchronous refresh detection that checks local executable availability, resolved path, current version, latest npm version, and available npm versions.
- Filter available versions to the latest 20 stable versions by default, excluding prerelease versions.
- Add asynchronous install, upgrade, and downgrade operations using backend-owned npm global install commands.
- Show the most recent operation state and expandable logs inside each CLI card.

## Capabilities

### Modified Capabilities

- `settings-center-ui`: Replaces the provider management page with the CLI management page and defines its UI states and interactions.
- `agent-tool-registry`: Extends supported agent CLI metadata and status with persisted local detection and npm version information.
- `native-runtime-architecture`: Requires CLI detection and CLI package operations to run as asynchronous backend-managed operations with guarded command construction.

## Impact

- Affects both the Tauri desktop frontend and browser Web runtime because both render the same React settings center.
- Adds or extends frontend service interfaces and runtime adapters; React components must keep using services rather than calling Tauri `invoke()` directly.
- Adds Tauri/Rust behavior for persisted CLI detection results, npm version discovery, and npm global install operations.
- Adds SQLite persistence for last-known CLI detection results so initial page rendering does not block on external commands.
- Web runtime cannot truly inspect local CLI tools; its adapter must expose unsupported or unavailable native detection behavior without faking installed CLI state.
- Does not add provider API Key, URL, preset, enable, edit, delete, or provider persistence behavior.

## Why

The session workspace now uses a real CLI terminal, but the visible naming still says Agent Terminal and launch failures for Claude Code/Codex CLI do not expose enough diagnostic context in the side panel. Windows CLI startup also needs to tolerate package-manager shim paths so configured CLI executables launch consistently.

## What Changes

- Rename the user-facing Agent Terminal session tab to Workspace / 工作区.
- Rename the right-side Agent Info tab to Basic Info / 基本信息.
- Add a Logs tab to the right information panel that shows recent session logs, including Agent terminal startup and failure diagnostics.
- Add a command input bar under the Agent CLI workspace terminal for users who prefer form-style input in addition to direct terminal typing.
- Improve native Agent terminal launch diagnostics so wrapper generation, PTY creation, and spawn failures are recorded before returning an error.
- Resolve known Windows npm-style CLI shim executables for managed interactive agents when a real package binary is present.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `main-layout-ui`: Update session workspace and information-panel labels, and add the compact session log tab.
- `agent-terminal-runtime`: Require launch failure diagnostics and broader Windows shim normalization for managed CLI terminal starts.

## Impact

- Affects both desktop and Web UI text/rendering through existing React service boundaries.
- Desktop/Tauri runtime changes are scoped to the native Agent terminal process launcher and unified session logs.
- No new dependencies, no database migration, and no direct Tauri `invoke()` calls from React components.

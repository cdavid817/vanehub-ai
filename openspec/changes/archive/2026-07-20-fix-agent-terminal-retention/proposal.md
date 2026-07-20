## Why

Switching back to an active Claude Code CLI session can start another PowerShell wrapper instead of reliably reusing the retained terminal process. The existing idle cleanup window is also shorter than the desired two-hour active-session retention.

## What Changes

- Extend retained Agent Terminal inactivity cleanup from 30 minutes to two hours.
- Prevent concurrent same-session Agent Terminal open requests from spawning duplicate CLI wrapper processes.
- Replay retained terminal output when the user returns to a live Agent Terminal session.
- Render a bounded frontend terminal replay cache immediately on session tab remount, before the native attach response returns.
- Keep the behavior behind the existing React service, Tauri adapter, and Rust native runtime boundaries.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `agent-terminal-runtime`: Require two-hour retained terminal idle cleanup, same-session duplicate spawn prevention, retained terminal output replay, and immediate frontend cached replay.
- `native-runtime-architecture`: Clarify that the terminal registry serializes same-session open requests before process launch.

## Impact

- Affects desktop/Tauri Agent Terminal runtime behavior.
- Web/mock runtime contract is unchanged.
- No new dependencies, no database migration, and no direct Tauri `invoke()` calls from React components.

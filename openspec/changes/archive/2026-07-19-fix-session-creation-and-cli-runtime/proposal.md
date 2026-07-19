## Why

Session creation currently exposes low-level interaction choices, has poor contrast in local/remote selection, generates confusing path/name defaults on Windows, and does not reliably route Codex CLI chat messages. These issues block the core workflow of creating a usable agent session and sending the first message.

## What Changes

- Remove `cli` and `native-desktop` as direct user-facing choices from the create-session page.
- Make local/remote create-session choices readable in selected and unselected states.
- Normalize Windows extended-length paths such as `\\?\D:\...` before showing paths in UI or deriving session names.
- Default new session names to `<current-folder-name>-<timestamp>`.
- Change opening-method management so order/default selection can be changed without launching an external program.
- Diagnose and fix Codex CLI message routing so created Codex sessions produce responses or visible failures.
- Show the selected CLI tool icon on the session page.

## Capabilities

### New Capabilities

### Modified Capabilities

- `session-management`: Creation defaults and visible session metadata are adjusted.
- `interaction-modes`: User-facing creation choices no longer expose implementation mode identifiers.
- `workspace-folder-openers`: Opening-method ordering/default changes must be configuration-only.
- `session-runtime-management`: Codex CLI runtime routing must support sent messages and expose failures.
- `chat-experience`: Session pages must display the selected CLI icon and keep first-message feedback visible.

## Impact

- Affects both desktop runtime and Web/mock runtime adapters where session defaults and UI state are represented.
- Frontend changes are scoped to React pages/components and service-boundary types.
- Backend changes are scoped to Tauri session/runtime/opening-method commands and Rust domain adapters if required.
- No new dependencies are expected.

## Why

CLI and Agent choices currently follow different source-array orders across settings and session creation, so users must repeatedly scan for the same tools in different positions. A shared, deterministic priority makes these surfaces predictable without changing stable Agent ids or runtime behavior.

## What Changes

- Order Agent choices throughout settings as Claude Code, Codex CLI, OpenCode, Antigravity CLI, Gemini CLI, then OnePiece.
- Order built-in CLI choices in create-session surfaces as Claude Code, Codex CLI, OpenCode, Antigravity CLI, then Gemini CLI; OnePiece remains in its separate native group.
- Preserve the relative source order of custom or future Agents after the recognized priority entries.
- Add unit, component, and browser regression coverage for the two ordering contexts.

## Capabilities

### New Capabilities
- `agent-ui-ordering`: Defines the shared settings priority, create-session CLI priority, and stable fallback behavior for unrecognized Agents.

### Modified Capabilities
- `settings-cli-management-ui`: Changes the fixed CLI management card order to the shared settings priority.
- `session-management`: Changes the built-in CLI ordering and default candidate used by create-session UI.

## Impact

- Affects React settings pages and the create-session UI in both desktop and Web runtimes.
- Adds a frontend ordering utility and updates existing frontend Agent-id collections; it does not change service contracts, native commands, persistence, or adapter boundaries.
- No backend migration or compatibility behavior is required.

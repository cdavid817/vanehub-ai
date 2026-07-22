## Why

Session restoration currently does not reliably carry the provider resume identifier from initial creation into later terminal starts, so reopened sessions can lose historical CLI context. The session sidebar also becomes hard to scan as projects and sessions grow because it has a fixed width and a flat session list.

## What Changes

- Persist the provider resume/runtime session id when a CLI-backed session is created or when the runtime reports it.
- Use the persisted resume id when reopening a session so the Agent CLI restores historical conversation context when supported by the provider.
- Add a draggable resize handle to the session sidebar with bounded widths and stable layout behavior.
- Group sessions by project in the sidebar, using workspace/project path metadata and an ungrouped bucket for sessions without a project.
- Keep desktop and Web/mock adapters behaviorally aligned for session metadata, resume ids, and grouped sidebar rendering.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `agent-terminal-runtime`: Clarify resume id capture during session creation and restore behavior when reopening CLI terminal sessions.
- `session-management`: Add project-based session grouping expectations for session list consumers.
- `main-layout-ui`: Add resizable session sidebar behavior.

## Impact

- Frontend UI: `src/main-layout/*` sidebar and layout state, plus related tests.
- Frontend service boundary: preserve existing `Session.runtimeSessionId` and project metadata across Tauri and Web/mock adapters.
- Desktop runtime: session creation, terminal start/resume flow, SQLite-backed runtime session id persistence.
- Web/mock runtime: mock session creation and restore behavior for parity.
- No new dependencies are expected.

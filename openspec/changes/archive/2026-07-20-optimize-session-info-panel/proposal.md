## Why

The workspace information panel still reflects an earlier prototype structure with Files, Changes, Logs, and a hard-coded progress summary, while current CLI sessions need a concise operational view of the selected model, usage, and active Skills. Optimizing this panel makes the right side of the workspace useful during real Agent CLI work without forcing users to leave the main workspace.

## What Changes

- Replace the information panel tab set with Basic Info, Token Usage, and Skill tabs.
- Update Basic Info to show the active session's CLI identity, lifecycle state, project/worktree context, and the model id from the session chat configuration.
- Add session-scoped usage presentation that prefers reported token totals; when no reported tokens exist, show a localized no-reported-token state with estimated response and character context.
- Add Skill presentation grouped into available Skills for the selected CLI and project Skills discovered for the active workspace, with disabled project Skills visually de-emphasized and kept separate from the available group.
- Preserve panel collapse behavior, keep-alive tab state, internal scrolling, translated zh-CN/en labels, and theme-token styling for both `futuristic` and `minimal` styles.
- Support both desktop and Web/mock runtimes through the frontend service boundary; React components must not call Tauri APIs directly.

## Capabilities

### New Capabilities

### Modified Capabilities
- `main-layout-ui`: Change the information panel tabs and define the Basic Info, Token Usage, and Skill panel behavior.
- `usage-statistics`: Add session-scoped usage summary semantics for workspace panels.
- `frontend-runtime-architecture`: Extend the Agent service and runtime adapter parity expectations for session-scoped usage data consumed by React.

## Impact

- Frontend UI: `src/main-layout/session-info-panel.tsx`, related tests, and workspace i18n resources.
- Frontend service boundary: `src/services/agent-service.ts`, `src/services/tauri-agent-client.ts`, `src/services/web-agent-client.ts`, and shared TypeScript/contract tests if the session usage contract is added.
- Native desktop runtime: a Tauri command and sessions usage repository/application path for session-scoped usage aggregation.
- Web runtime: deterministic mock session usage aggregation compatible with the desktop contract.
- OpenSpec: delta specs for `main-layout-ui`, `usage-statistics`, and `frontend-runtime-architecture`.

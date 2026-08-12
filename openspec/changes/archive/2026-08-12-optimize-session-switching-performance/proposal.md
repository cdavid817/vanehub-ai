## Why

Switching between sessions from the sidebar currently waits for the persistence call and triggers broad query invalidation before the newly selected conversation becomes stable. The visible delay and unnecessary workspace remounts make a common navigation action feel sluggish, especially when users move repeatedly between active sessions.

## What Changes

- Update the selected session optimistically from the already-loaded sidebar record while persistence continues asynchronously.
- Roll back the visible selection if persistence fails and prevent an older switch response from replacing a newer user selection.
- Replace broad session-query invalidation with targeted cache reconciliation for active-session and session-list data.
- Reset session-scoped workspace state only when the effective active session actually changes.
- Add deterministic component/model tests and Playwright timing coverage for rapid session switching and failure rollback.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `main-layout-ui`: Require responsive, race-safe session selection that immediately reflects an already-loaded sidebar session and avoids unnecessary workspace resets.

## Impact

- Affects the shared React main-layout model and session sidebar behavior in both Tauri desktop and Web runtimes.
- Uses the existing `AgentService.switchSession` boundary; no component will call Tauri APIs directly and no backend or service contract change is required.
- Updates frontend query-cache behavior, model tests, and Playwright coverage without adding dependencies.

## Why

Users need VaneHub AI to run recurring Agent work without manually creating the same session and prompt each time. Scheduled tasks provide a first-class way to define recurring prompts, track their status, and resume missed app-closed runs predictably.

## What Changes

- Add a Scheduled Tasks entry point that opens a scheduled-task management dialog instead of a coming-soon placeholder.
- Add scheduled task creation with task name, task content, selected Agent tool, frequency type, and frequency parameters.
- Support minute, hourly, daily, weekly, and monthly recurrence.
- Add a task list showing created scheduled tasks with enabled state, next run time, latest status, and actions to enable/disable or delete tasks.
- Execute each due task by creating a new session for the selected Agent and sending the configured task content to that session.
- Run scheduled tasks while VaneHub AI is open; on startup, backfill at most one missed run per enabled task and recompute the next run time from the current time.
- Keep desktop persistence and execution in the native/service layers while preserving Web/mock adapter parity.

## Capabilities

### New Capabilities
- `scheduled-task-management`: Defines scheduled task configuration, listing, mutation, recurrence, due-run execution, startup backfill, and service boundary behavior.

### Modified Capabilities
- `main-layout-ui`: Replace the Scheduled Tasks activity placeholder with the management dialog entry point.

## Impact

- Frontend service boundary: add scheduled task listing, creation, update, deletion, and task-run operations to `src/services/agent-service.ts`.
- Desktop adapter: add Tauri-backed scheduled task methods in `src/services/tauri-agent-client.ts`.
- Web/mock adapter: add equivalent mock scheduled task behavior in `src/services/web-agent-client.ts`.
- React UI: add scheduled task management dialog and connect the existing activity-bar entry to it.
- Rust/Tauri runtime: persist scheduled tasks in SQLite, expose commands behind the Tauri adapter, compute due/backfill runs, create sessions, and send task content through the existing session chat runtime.
- Logging: scheduled task execution, failure, deletion, and startup backfill diagnostics must use unified log management.

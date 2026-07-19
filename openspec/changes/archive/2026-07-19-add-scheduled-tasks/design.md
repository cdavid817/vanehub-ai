## Context

The main workspace already has a Scheduled Tasks activity-bar entry, but the current `main-layout-ui` specification treats it as a localized coming-soon placeholder. Session creation and chat execution already live behind the frontend agent service and runtime-specific adapters, with real desktop execution in Rust/Tauri and mock parity in Web mode.

Scheduled tasks cross the UI, frontend service boundary, SQLite persistence, startup lifecycle, and session chat runtime. The feature must therefore avoid React-owned scheduling or direct Tauri calls, and must use stable Agent ids instead of display-name matching.

## Goals / Non-Goals

**Goals:**
- Provide a scheduled-task management dialog from the existing activity-bar entry.
- Let users create tasks with name, content, Agent selection, recurrence, enabled state, and visible run status.
- Execute due tasks by creating a new session and submitting the stored task content to the selected Agent.
- Persist scheduled tasks in the desktop runtime and preserve equivalent Web/mock behavior.
- Backfill only the most recent missed run at application startup, then compute the next run from the current time.
- Keep diagnostics and operation history routed through unified logging.

**Non-Goals:**
- No OS-level background service, daemon, or wake-from-sleep scheduling.
- No execution while VaneHub AI is closed.
- No full cron expression editor in the first version.
- No bulk catch-up of every missed interval after long downtime.
- No separate scheduled-task output viewer; generated sessions remain the primary place to inspect results.

## Decisions

### Use a native-owned scheduler for desktop execution

The desktop runtime will persist scheduled tasks in SQLite and run due-task checks from Rust-owned background work while the app is open. React will only display and mutate task configuration through the agent service boundary.

Alternative considered: run timers in React. This was rejected because React timers stop with component lifecycle, cannot reliably handle startup backfill, and would push persistence/execution policy into the UI.

### Represent recurrence as typed frequency settings, not raw cron

The first version will support `minutes`, `hours`, `daily`, `weekly`, and `monthly` recurrence kinds with structured parameters:
- minutes: interval in minutes.
- hours: interval in hours.
- daily: time of day.
- weekly: weekday plus time of day.
- monthly: day of month plus time of day.

Alternative considered: expose cron syntax. This was rejected for first version because it increases validation complexity and creates a poor fit for the requested dialog controls.

### Execute task runs through the existing session and chat path

When a task is due, the runtime will create a new session for the task's stable Agent id and submit the configured task content as the initial user message. The created session becomes part of the normal session list and uses existing chat persistence, streaming, failure, and logging behavior.

Alternative considered: store task output separately. This was rejected because it duplicates chat history and fragments result review.

### Backfill at most one missed run on startup

On startup, each enabled task will be checked for missed due time. If one or more runs were missed while the app was closed, the runtime will enqueue one catch-up run, record that it is a backfill, and compute the next run time from the current time.

Alternative considered: replay every missed interval. This was rejected because a task with a short interval could create many sessions after a long closure.

### Preserve Web/mock adapter parity without claiming native execution

The Web adapter will expose the same scheduled-task contracts with in-memory or mock persistence for local browser use. It may simulate run status deterministically, but it must not claim real local CLI execution.

## Risks / Trade-offs

- Missed local clock changes could make `nextRunAt` inaccurate -> Recompute next run from the current wall-clock time after each run and startup backfill.
- A due task could target an unavailable Agent -> Preserve the task, record a failed latest status, create or mark the attempted session according to the existing chat failure contract, and write diagnostics through unified logging.
- Monthly schedules can target invalid days for some months -> Clamp or skip according to a documented recurrence helper; the implementation should make this deterministic and covered by tests.
- Task-triggered sessions could surprise users if many tasks become due at once -> Run only enabled due tasks, backfill only one missed run per task, and show latest status plus next run time in the task list.
- Web/mock behavior could diverge from desktop -> Keep types and service methods shared at the frontend boundary and cover the Web adapter with focused tests.

## Migration Plan

- Add an additive SQLite migration for scheduled task records and latest-run metadata.
- Default existing installs to no scheduled tasks.
- Add Tauri commands and frontend adapter methods without removing existing session/chat contracts.
- Replace the Scheduled Tasks coming-soon behavior with the dialog entry point.
- Rollback can leave the scheduled task table unused; no existing session data is modified by the schema addition.

## Open Questions

- None for the first version. The confirmed execution model is: create a new session and send the task content to the selected Agent; task management includes list, enable/disable, delete, next run time, and latest status.

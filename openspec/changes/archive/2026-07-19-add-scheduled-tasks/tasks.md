## 1. Contracts and Data Model

- [x] 1.1 Add shared TypeScript scheduled task types, recurrence types, status types, and service method signatures to `src/services/agent-service.ts`.
- [x] 1.2 Add Rust scheduled task domain models and Tauri command request/response DTOs using stable Agent ids.
- [x] 1.3 Add an additive SQLite migration for scheduled tasks, recurrence configuration, next run time, latest status, and latest run metadata.
- [x] 1.4 Implement recurrence validation and next-run calculation helpers for minute, hourly, daily, weekly, and monthly schedules.

## 2. Desktop Runtime

- [x] 2.1 Implement Rust persistence operations to list, create, update enabled state, delete, and update run metadata for scheduled tasks.
- [x] 2.2 Expose scheduled task Tauri commands and wire them through the desktop command registration path.
- [x] 2.3 Implement native-owned due-task scheduling while the app is open.
- [x] 2.4 Implement startup missed-run detection that backfills at most one missed run per enabled task and recomputes next run from current time.
- [x] 2.5 Execute due task runs by creating a new session for the task's stable Agent id and submitting task content through the existing session chat runtime.
- [x] 2.6 Route task run start, completion, failure, skipped, delete, and backfill diagnostics through unified log management.

## 3. Frontend Service Adapters

- [x] 3.1 Implement scheduled task methods in `src/services/tauri-agent-client.ts` with Tauri `invoke()` calls kept inside the adapter.
- [x] 3.2 Implement equivalent Web/mock scheduled task behavior in `src/services/web-agent-client.ts` without claiming native CLI execution.
- [x] 3.3 Add focused unit coverage for recurrence formatting, Web adapter mutation behavior, and service-level validation where existing test patterns support it.

## 4. Workspace UI

- [x] 4.1 Replace the Scheduled Tasks activity-bar coming-soon action with a scheduled-task management dialog opener.
- [x] 4.2 Build the scheduled-task dialog with task list, create/edit form fields, localized placeholders, Agent selection, recurrence controls, and save validation.
- [x] 4.3 Show each created task's name, Agent, frequency summary, enabled state, next run time, latest status, and enable/disable/delete actions.
- [x] 4.4 Add localized zh-CN and en strings for scheduled task entry labels, dialog text, placeholders, validation errors, statuses, and confirmations.
- [x] 4.5 Ensure the dialog uses the frontend service boundary and never calls Tauri APIs directly from React components.

## 5. Verification

- [x] 5.1 Add or update Playwright coverage for opening the scheduled-task dialog, creating a task, toggling enabled state, and deleting a task in Web/mock mode.
- [x] 5.2 Add Rust tests for recurrence calculation, startup backfill behavior, disabled-task skipping, and latest status updates.
- [x] 5.3 Run `npm run test`.
- [x] 5.4 Run `npm run build`.
- [x] 5.5 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 5.6 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 5.7 Run `openspec validate "add-scheduled-tasks" --strict`.

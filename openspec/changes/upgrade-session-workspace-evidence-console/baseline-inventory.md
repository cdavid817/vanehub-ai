# Baseline inventory (Tasks 0.3 / 0.4)

Recorded against `main` at `ee3eaf3f` before any implementation commit of this change. This is
the compatibility contract the migration must not break: every command name and serialized field
below is reachable from a shipped adapter today.

## Frontend service boundary

`src/services/agent-service.ts` is the single interface React may depend on. The session-workspace
surface is split across two adapter pairs:

| Surface | Tauri adapter | Web/mock adapter |
| --- | --- | --- |
| Files / Documents / Git / Logs / Shell / folder openers | `src/services/tauri-session-workspace-client.ts` | `src/services/web-session-workspace-client.ts` (+ `web-session-workspace-fixtures.ts`) |
| Review Center | `src/services/tauri-agent-client.ts` | `src/services/web-code-review-client.ts` |
| Traces | `src/services/tauri-execution-observability-client.ts` | `src/services/web-execution-observability-client.ts` |
| Usage / Report inputs | `src/services/tauri-usage-statistics.ts` | `src/services/web-usage-statistics-client.ts` |

## Tauri command names that must stay compatible

Session workspace (`src-tauri/src/commands/workspaces/`):

```text
list_session_directory      read_session_file        list_session_documents
search_session_files        get_session_git_status   get_session_git_diff
list_session_logs           export_session_logs      inspect_project
list_known_projects         list_known_remote_workspaces
select_project_directory
shell_create   shell_input   shell_cd   shell_resize   shell_kill
```

Folder openers: `list_folder_openers`, `refresh_folder_openers`, `get_folder_opener_preferences`,
`save_folder_opener_preferences`, `open_session_folder`.

Execution observability (`src/services/tauri-execution-observability-client.ts`):

```text
get_observability_settings  update_observability_settings
list_execution_runs         get_execution_run
get_execution_timeline      get_execution_observation_capabilities
```

Review (`src/services/tauri-agent-client.ts:237-264`):

```text
open_code_review            get_code_review             load_code_review_file
add_code_review_comment     resolve_code_review_comment select_code_review_comment
set_code_review_decision    revert_code_review_change   send_code_review_feedback
start_code_review_action
```

`set_code_review_decision` is review-scoped. Task 1.7 adds a separate hunk-scoped command rather
than overloading this one, because hunk Accept currently routes through it
(`src/session-workspace/review-center.tsx:87`).

## Tauri event names currently consumed by React

```text
shell:event            folder-openers:event   agent-terminal:event
chat:event             session:event          settings:event
permission:request     builtin-tool-operation im-connector:lifecycle
floating-assistant:event
```

No session-log, evidence, or workspace-invalidation event exists yet; those are added by this
change and must not reuse the names above.

## Serialized DTO shapes that must remain readable

`src-tauri/src/commands/workspaces/dto.rs` serializes camelCase. The shapes React parses today:

- `BoundedResult<T>` — `{ items, truncated, nextCursor }`. `nextCursor` is currently an integer
  offset rendered as a string for logs; the keyset work in Task Groups 3 and 8 must keep the field
  name and its opaque-string type while changing its contents.
- `SessionWorkspaceContext` — `{ availability, rootName, reason }`.
- `DirectoryListing` — `BoundedResult<DirectoryEntry> & { context, path }`.
- `FileContent` — `{ path, name, status, size, content }`.
- `GitStatusResult` — `BoundedResult<GitStatusEntry> & { context, isGit, branch }`.
- `GitDiffResult` — `{ context, source, files, truncated }`.
- `SessionLogEntry` — `{ id, timestamp, level, category, message, context }`.
- `ShellSession` — `{ shellId, sessionId, state, capability }`. `capability` is a bare string that
  Rust already populates with `remote` (`workspaces/application/shell_service.rs:89`) while the
  TypeScript union admits only `native | simulated`. Task 1.1 replaces the field with a
  discriminated descriptor; the command name `shell_create` stays.
- `ShellEvent` — `{ type: "output" | "state", shellId, sessionId, ... }` on the `shell:event`
  channel.

## SQLite migration ledger

`src-tauri/src/platform/database/migrations/mod.rs` holds `EXPECTED_MIGRATIONS`, dense from 1 to
**80** (`retire-plan-execution`). New migrations for this change start at 81 and must be appended
in lockstep with `apply_migration` calls, because `assert_migration_history_is_dense` fails
startup on a gap and `migration_sequence_matches_expected` fails on name drift. Version numbers
must be checked against other open branches before allocation: every worktree shares one
`ai.vanehub.app` database, so a collision surfaces as a missing table at startup rather than as a
migration error.

## Test baseline

`npm run test` on `ee3eaf3f` in this worktree: 296 files, 1350 tests, 0 failures.

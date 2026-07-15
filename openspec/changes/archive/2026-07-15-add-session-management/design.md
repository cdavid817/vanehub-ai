## Context

VaneHub currently models workflow state as a single active agent and interaction mode, while the main-layout sidebar presents static mock session cards. That prevents users from managing the normal developer workflow of starting multiple agent sessions, returning to prior work, pinning important sessions, or archiving completed ones.

This change spans the shared TypeScript service contract, both runtime adapters, the Tauri/Rust persistence layer, and sidebar UI behavior. React components must continue to call only the `AgentService` interface; Tauri `invoke()` calls stay in `tauri-agent-client.ts`, and SQLite access stays in Rust.

## Goals / Non-Goals

**Goals:**

- Introduce a durable `Session` entity that can represent multiple agent workflows across app restarts.
- Keep desktop and Web runtimes aligned through the same `AgentService` methods.
- Store desktop sessions in SQLite and track the selected active session.
- Let users create, switch, rename, pin, unpin, archive, restore, and delete sessions from the sidebar.
- Keep archived sessions recoverable and visible through explicit archived-session behavior.

**Non-Goals:**

- Persist full chat transcripts, terminal output, file diffs, or agent execution logs.
- Implement folder creation or folder management beyond storing a session folder label.
- Start or stop external agent processes as part of session create, switch, archive, or delete actions.
- Add a remote HTTP backend for Web mode.
- Replace the existing workflow-state model for launch and readiness behavior.

## Decisions

### 1. Add a dedicated `sessions` table with UUID primary keys

Sessions will be stored in a new SQLite table with a text UUID primary key and columns for title, agent id, interaction mode, lifecycle state, folder, pinned, archived, created timestamp, and updated timestamp.

Rationale: sessions need independent identity from the current singleton workflow row, and UUIDs avoid coupling future session records to SQLite row ids or agent-specific identifiers. This also gives the Web adapter a simple id shape that matches desktop behavior.

Alternative considered: reuse the existing `workflow_state` row as the session record. That would preserve today only one session and would make switching, archiving, and deletion ambiguous.

### 2. Reuse `workflow_state` for active runtime state and add `active_session_id`

The existing `workflow_state` table remains the source for the currently selected agent, interaction mode, lifecycle state, and intent. A nullable `active_session_id` column links that singleton runtime state to the selected session.

Rationale: workflow state already represents active runtime selection, so replacing it would create unnecessary churn across launch and readiness paths. Linking the active session keeps the migration additive.

Alternative considered: make every workflow-state read join through `sessions` and remove the singleton state. That is a larger architecture change and would blur session management with agent launch state.

### 3. Keep the session API at the `AgentService` boundary

`AgentService` will expose list, create, switch, rename, pin, archive, restore, and delete methods. React components will consume those methods through the existing service context. The Tauri adapter will call Rust commands with `invoke()`, while the Web adapter will use in-memory mock state.

Rationale: this preserves the runtime boundary and keeps the sidebar independent of Tauri. The Web runtime remains usable and can later swap its in-memory implementation for HTTP without changing components.

Alternative considered: have the sidebar call Tauri commands directly for speed. That violates the existing architecture constraint and would break browser mode.

### 4. Treat archiving as reversible organization, not deletion

Archiving sets `archived = true`, updates `updated_at`, and clears `active_session_id` if the archived session was active. Archived sessions remain queryable through `listArchivedSessions()` and may be restored through `unarchiveSession()`.

Rationale: users need a low-risk way to clean up the sidebar without losing work context. Clearing the active session prevents the app from pointing at a hidden archived session.

Alternative considered: delete on archive. That makes archive irreversible and duplicates delete semantics.

### 5. Use `"新会话"` as the default created-session title

`createSession()` will create a session with the default title `"新会话"` when the caller does not provide a title.

Rationale: the UI can make the new-session action immediate and predictable without forcing a naming dialog before the record exists. Users can rename after creation.

Alternative considered: derive a title from the agent or timestamp. That is less clear for Chinese UI and makes repeated sessions visually noisy.

### 6. Sidebar grouping is derived from session metadata

Pinned sessions render in a dedicated pinned area. Activity grouping is derived from `Session.lifecycleState` and `archived`, with archived sessions mapped to inactive unless the archived view is active. Folder grouping uses the session `folder` field. Context actions operate on the selected session record.

Rationale: this keeps UI behavior deterministic and avoids storing duplicate grouping state. Session cards remain a presentation of service data rather than a separate UI-only model.

Alternative considered: maintain separate sidebar conversation mock data. That would duplicate state and keep the "new" and card selection behavior disconnected from persistence.

## Risks / Trade-offs

- Schema migration fails on an existing database -> Keep migration additive, idempotent, and versioned.
- Desktop and Web behavior diverge -> Define all operations in `AgentService` and implement the same method set in both adapters.
- Deleting or archiving the active session leaves stale UI state -> Clear `active_session_id` when the active session is archived or deleted and refresh session queries after mutations.
- A session lifecycle state can drift from real agent process state -> Treat lifecycle as stored metadata for this change; process supervision remains outside scope.
- Sidebar becomes crowded when archived sessions are included in normal lists -> Keep an explicit archived view and make archived ordering predictable.
- New Rust dependencies increase build surface -> Use narrow dependencies for UUID generation and timestamp serialization only.

## Migration Plan

1. Add a new database migration version that creates `sessions` and adds nullable `active_session_id` to `workflow_state`.
2. Register the session Tauri commands and expose matching TypeScript methods in both runtime adapters.
3. Update the sidebar to read sessions from `AgentService`, perform mutations through service methods, and refresh cached session state after mutations.
4. Verify the change with OpenSpec validation, frontend build/type checks, and Rust checks.

Rollback is limited because SQLite schema migrations are additive. If the feature is disabled in code, existing `sessions` rows and the nullable `active_session_id` column can remain unused without breaking older workflow-state behavior.

## Open Questions

- Should future work migrate existing singleton workflow state into an initial session on first launch?
- Should session folder values become a managed entity with create, rename, and delete operations?
- Should session lifecycle state eventually be synchronized with process supervision instead of being stored as UI metadata?
